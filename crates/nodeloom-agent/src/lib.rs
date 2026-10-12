//! The Nodeloom agent: connects to a master, reports resource usage and runs
//! the tasks it is assigned.

use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use nodeloom_core::{
    RunnerSet,
    metrics::{MetricsCollector, describe_node},
    validate::validate_payload,
};
use nodeloom_proto::{AgentMessage, MasterMessage, NodeId, PROTOCOL_VERSION, TaskId, TaskSpec};
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

pub struct AgentConfig {
    /// Full URL of the master's agent endpoint, e.g. `ws://192.168.1.5:7878/ws/agent`.
    pub master_url: String,
    pub token: String,
    pub node_id: NodeId,
    pub runners: RunnerSet,
    pub metrics_interval: Duration,
}

/// Events produced by running tasks and consumed by the connection loop.
enum TaskEvent {
    Progress {
        task_id: TaskId,
        done: u32,
        total: u32,
    },
    Done {
        task_id: TaskId,
    },
    Failed {
        task_id: TaskId,
        error: String,
    },
}

/// Connects once and serves until the connection ends. Returns `Ok(())` when
/// the master closes the connection cleanly.
pub async fn run_once(config: &AgentConfig) -> Result<()> {
    let (socket, _) = tokio_tungstenite::connect_async(&config.master_url)
        .await
        .with_context(|| format!("connecting to {}", config.master_url))?;
    let (mut sink, mut stream) = socket.split();

    let hello = AgentMessage::Hello {
        protocol: PROTOCOL_VERSION,
        token: config.token.clone(),
        info: describe_node(config.node_id.clone(), config.runners.kinds()),
    };
    sink.send(Message::text(serde_json::to_string(&hello)?))
        .await?;

    match next_master_message(&mut stream).await? {
        Some(MasterMessage::Welcome) => info!("paired with master"),
        Some(MasterMessage::Rejected { reason }) => bail!("master rejected us: {reason}"),
        other => bail!("unexpected first message from master: {other:?}"),
    }

    let (event_tx, mut event_rx) = mpsc::channel::<TaskEvent>(256);
    let mut running: HashMap<TaskId, JoinHandle<()>> = HashMap::new();
    let mut collector = MetricsCollector::new();
    let mut ticker = tokio::time::interval(config.metrics_interval);

    let result = loop {
        tokio::select! {
            _ = ticker.tick() => {
                let msg = AgentMessage::Metrics(collector.sample());
                if let Err(e) = sink.send(Message::text(serde_json::to_string(&msg)?)).await {
                    break Err(e.into());
                }
            }
            Some(event) = event_rx.recv() => {
                let msg = match event {
                    TaskEvent::Progress { task_id, done, total } => AgentMessage::TaskProgress {
                        task_id, frames_done: done, frames_total: total,
                    },
                    TaskEvent::Done { task_id } => {
                        running.remove(&task_id);
                        AgentMessage::TaskDone { task_id }
                    }
                    TaskEvent::Failed { task_id, error } => {
                        running.remove(&task_id);
                        AgentMessage::TaskFailed { task_id, error }
                    }
                };
                if let Err(e) = sink.send(Message::text(serde_json::to_string(&msg)?)).await {
                    break Err(e.into());
                }
            }
            incoming = next_master_message(&mut stream) => match incoming {
                Ok(Some(MasterMessage::AssignTask(spec))) => {
                    let task_id = spec.task_id;
                    let handle = spawn_task(spec, config.runners.clone(), event_tx.clone());
                    running.insert(task_id, handle);
                }
                Ok(Some(MasterMessage::CancelTask { task_id })) => {
                    if let Some(handle) = running.remove(&task_id) {
                        handle.abort();
                    }
                }
                Ok(Some(other)) => warn!("ignoring unexpected message: {other:?}"),
                Ok(None) => break Ok(()),
                Err(e) => break Err(e),
            },
        }
    };

    for (_, handle) in running {
        handle.abort();
    }
    result
}

/// Reconnects forever with exponential backoff (1s up to 30s).
pub async fn run_forever(config: AgentConfig) -> ! {
    let mut delay = Duration::from_secs(1);
    loop {
        match run_once(&config).await {
            Ok(()) => info!("master closed the connection"),
            Err(e) => warn!("connection ended: {e:#}"),
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_secs(30));
    }
}

fn spawn_task(
    spec: TaskSpec,
    runners: RunnerSet,
    events: mpsc::Sender<TaskEvent>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let task_id = spec.task_id;
        let outcome = async {
            validate_payload(&spec.payload).map_err(|e| format!("invalid task: {e}"))?;
            let runner = runners
                .get(spec.payload.kind())
                .map_err(|e| e.to_string())?;
            let total = spec.payload.frame_count();

            let (progress_tx, mut progress_rx) = mpsc::channel::<u32>(64);
            let forward = {
                let events = events.clone();
                async move {
                    while let Some(done) = progress_rx.recv().await {
                        let _ = events
                            .send(TaskEvent::Progress {
                                task_id,
                                done,
                                total,
                            })
                            .await;
                    }
                }
            };
            let run = async {
                let result = runner.run(&spec.payload, &progress_tx).await;
                drop(progress_tx);
                result
            };
            let (result, ()) = tokio::join!(run, forward);
            result.map_err(|e| e.to_string())
        }
        .await;

        let event = match outcome {
            Ok(()) => TaskEvent::Done { task_id },
            Err(error) => TaskEvent::Failed { task_id, error },
        };
        let _ = events.send(event).await;
    })
}

async fn next_master_message<S>(stream: &mut S) -> Result<Option<MasterMessage>>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Text(text) => return Ok(Some(serde_json::from_str(text.as_str())?)),
            Message::Close(_) => return Ok(None),
            _ => {} // pings are answered by tungstenite itself
        }
    }
    Ok(None)
}
