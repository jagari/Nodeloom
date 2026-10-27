//! The Nodeloom agent: connects to a master, proves its identity, reports
//! resource usage and runs the tasks it is assigned.

pub mod identity;

use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use nodeloom_core::{
    RunnerSet,
    metrics::{MetricsCollector, describe_node},
    validate::validate_payload,
};
use nodeloom_proto::{AgentMessage, MasterMessage, PROTOCOL_VERSION, TaskId, TaskSpec};
use serde::Serialize;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

pub use identity::Identity;

/// While waiting for approval the master pings every 15 seconds. Hearing
/// nothing for this long means the connection is dead; reconnect.
const PENDING_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

pub struct AgentConfig {
    /// Full URL of the master's agent endpoint, e.g. `ws://192.168.1.5:7878/ws/agent`.
    pub master_url: String,
    pub identity: Identity,
    /// The master's admin token, to join without approval in the dashboard.
    pub token: Option<String>,
    pub runners: RunnerSet,
    pub metrics_interval: Duration,
    /// Where to report what the agent is doing, for a screen to show (the
    /// desktop app). The log says the same for the command-line agent.
    pub status: Option<watch::Sender<AgentStatus>>,
}

/// What the agent is doing right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AgentStatus {
    /// Looking for a master on the local network. The agent loop never
    /// reports this; callers that run discovery before it do.
    Searching,
    /// Connecting to `master`, or waiting to retry after `last_error`.
    Connecting {
        master: String,
        last_error: Option<String>,
    },
    /// Connected, but the master waits for someone to approve this machine
    /// in the dashboard after comparing `code`.
    WaitingForApproval { master: String, code: String },
    /// In the cluster and taking tasks.
    Joined { master: String },
}

impl AgentConfig {
    fn report(&self, status: AgentStatus) {
        if let Some(tx) = &self.status {
            tx.send_replace(status);
        }
    }
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
    session(config, &mut false).await
}

/// Like [`run_once`]; sets `welcomed` once the master has let us in.
async fn session(config: &AgentConfig, welcomed: &mut bool) -> Result<()> {
    let (socket, _) = tokio_tungstenite::connect_async(&config.master_url)
        .await
        .with_context(|| format!("connecting to {}", config.master_url))?;
    let (mut sink, mut stream) = socket.split();
    let identity = &config.identity;

    let hello = AgentMessage::Hello {
        protocol: PROTOCOL_VERSION,
        public_key: identity.key.public_key().as_hex().to_owned(),
        info: describe_node(identity.node_id.clone(), config.runners.kinds()),
        token: config.token.clone(),
    };
    sink.send(Message::text(serde_json::to_string(&hello)?))
        .await?;

    match next_master_message(&mut stream).await? {
        Some(MasterMessage::Challenge { nonce }) => {
            let proof = AgentMessage::Proof {
                signature: identity.key.sign_hello(&nonce, &identity.node_id),
            };
            sink.send(Message::text(serde_json::to_string(&proof)?))
                .await?;
        }
        Some(MasterMessage::Rejected { reason }) => bail!("master rejected us: {reason}"),
        None => bail!("master closed the connection during the handshake (different version?)"),
        Some(other) => bail!("unexpected first message from master: {other:?}"),
    }

    let mut announced = false;
    loop {
        match next_master_message_within(&mut stream, PENDING_IDLE_TIMEOUT).await? {
            Some(MasterMessage::Welcome) => break,
            Some(MasterMessage::Pending) => {
                if !announced {
                    config.report(AgentStatus::WaitingForApproval {
                        master: config.master_url.clone(),
                        code: identity.confirmation_code(),
                    });
                    info!(
                        "waiting for approval on {}. Confirmation code: {}. Approve this \
                         machine in the Nodeloom dashboard only if it shows the same code.",
                        config.master_url,
                        identity.confirmation_code()
                    );
                    announced = true;
                }
            }
            Some(MasterMessage::Rejected { reason }) => bail!("master rejected us: {reason}"),
            None => bail!("master closed the connection before deciding"),
            Some(other) => bail!("unexpected message during the handshake: {other:?}"),
        }
    }
    info!("joined the cluster at {}", config.master_url);
    *welcomed = true;
    config.report(AgentStatus::Joined {
        master: config.master_url.clone(),
    });

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
                // Sent when the machine is removed from the cluster.
                Ok(Some(MasterMessage::Rejected { reason })) => {
                    break Err(anyhow::anyhow!("master rejected us: {reason}"));
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

/// Reconnects forever with exponential backoff (1s up to 30s). A session
/// that got past the handshake resets the backoff, so a master restart is
/// followed by a quick reconnect.
pub async fn run_forever(config: AgentConfig) -> ! {
    let mut delay = Duration::from_secs(1);
    config.report(AgentStatus::Connecting {
        master: config.master_url.clone(),
        last_error: None,
    });
    loop {
        let mut welcomed = false;
        let error = match session(&config, &mut welcomed).await {
            Ok(()) => {
                info!("master closed the connection");
                "the master closed the connection".to_owned()
            }
            Err(e) => {
                warn!("connection ended: {e:#}");
                format!("{e:#}")
            }
        };
        // Shown while waiting to retry, so a rejection (say, a removed
        // machine) stays on screen instead of flashing past.
        config.report(AgentStatus::Connecting {
            master: config.master_url.clone(),
            last_error: Some(error),
        });
        if welcomed {
            delay = Duration::from_secs(1);
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

/// Like [`next_master_message`], but gives up when no frame at all (pings
/// included) arrives for `idle`.
async fn next_master_message_within<S>(
    stream: &mut S,
    idle: Duration,
) -> Result<Option<MasterMessage>>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let frame = match tokio::time::timeout(idle, stream.next()).await {
            Err(_) => bail!("heard nothing from the master for {}s", idle.as_secs()),
            Ok(None) => return Ok(None),
            Ok(Some(frame)) => frame?,
        };
        match frame {
            Message::Text(text) => return Ok(Some(serde_json::from_str(text.as_str())?)),
            Message::Close(_) => return Ok(None),
            _ => {}
        }
    }
}
