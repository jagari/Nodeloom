//! The Nodeloom agent: connects to a master, proves its identity, reports
//! resource usage and runs the tasks it is assigned.

pub mod identity;

use std::{collections::HashMap, fmt::Display, future::Future, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use futures_util::{SinkExt, StreamExt};
use nodeloom_core::{
    RunnerSet, discovery,
    metrics::{MetricsCollector, describe_node},
    validate::{fit_node_info, validate_payload},
};
use nodeloom_proto::{AgentMessage, MasterMessage, NodeInfo, PROTOCOL_VERSION, TaskId, TaskSpec};
use serde::Serialize;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::Instant,
};
use tokio_tungstenite::tungstenite::{self, Message};
use tracing::{info, warn};

pub use identity::Identity;

/// While waiting for approval the master pings every 15 seconds. Hearing
/// nothing for this long means the connection is dead; reconnect.
const PENDING_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// How long connecting and the first handshake step may take. Without it a
/// master that went away mid-connect (or accepts but never answers) would
/// hold the agent until the operating system gives up, which can be minutes.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Why a joined session ends when the master goes quiet.
const MASTER_SILENT: &str = "the master stopped answering";

/// Consecutive failed connections to a master found by discovery after
/// which [`run_forever_discovering`] looks for it again (it may have a new
/// address, say after the router gave it another IP).
pub const REDISCOVER_AFTER_FAILURES: u32 = 5;

/// Pause between discovery attempts in [`find_master_forever`].
const DISCOVERY_RETRY_PAUSE: Duration = Duration::from_secs(2);

/// Retry delays between sessions: doubled after each failure, up to `max`.
#[derive(Debug, Clone, Copy)]
struct Backoff {
    first: Duration,
    max: Duration,
}

const BACKOFF: Backoff = Backoff {
    first: Duration::from_secs(1),
    max: Duration::from_secs(30),
};

/// How often to ping a joined master, and how long it may stay completely
/// silent before the session is given up. Both follow `metrics_interval`
/// (no extra config field), so a test reporting every 50 ms notices a dead
/// master in seconds: ping every 5 intervals (1–5 s), give up after 15
/// (3–15 s).
fn keepalive(metrics_interval: Duration) -> (Duration, Duration) {
    let ping = metrics_interval
        .saturating_mul(5)
        .clamp(Duration::from_secs(1), Duration::from_secs(5));
    let idle = metrics_interval
        .saturating_mul(15)
        .clamp(Duration::from_secs(3), Duration::from_secs(15));
    (ping, idle)
}

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
    /// What one frame produced (an LLM's answer), sent as soon as it exists.
    Output {
        task_id: TaskId,
        frame: u32,
        text: String,
    },
}

/// Connects once and serves until the connection ends. Returns `Ok(())` when
/// the master closes the connection cleanly.
pub async fn run_once(config: &AgentConfig) -> Result<()> {
    session(config, &mut false).await
}

/// Like [`run_once`]; sets `welcomed` once the master has let us in.
async fn session(config: &AgentConfig, welcomed: &mut bool) -> Result<()> {
    let connect = tokio_tungstenite::connect_async(&config.master_url);
    let (socket, _) = match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
        Ok(result) => result.with_context(|| format!("connecting to {}", config.master_url))?,
        Err(_) => bail!(
            "connecting to {}: no answer within {}s",
            config.master_url,
            CONNECT_TIMEOUT.as_secs()
        ),
    };
    let (mut sink, mut stream) = socket.split();
    let identity = &config.identity;

    let hello = AgentMessage::Hello {
        protocol: PROTOCOL_VERSION,
        public_key: identity.key.public_key().as_hex().to_owned(),
        info: fit_node_info(NodeInfo {
            llm_models: config.runners.models(),
            ..describe_node(identity.node_id.clone(), config.runners.kinds())
        }),
        token: config.token.clone(),
    };
    sink.send(Message::text(serde_json::to_string(&hello)?))
        .await?;

    match next_master_message_within(&mut stream, CONNECT_TIMEOUT).await? {
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
    // A master that went to sleep or lost its Wi-Fi sends nothing, not even
    // a close, and the socket would wait forever. Ping it (it answers by
    // itself) and give up once nothing at all arrives for `idle_limit`.
    let (ping_every, idle_limit) = keepalive(config.metrics_interval);
    let mut pinger = tokio::time::interval_at(Instant::now() + ping_every, ping_every);
    let mut last_heard = Instant::now();

    let result = loop {
        let silent_until = last_heard + idle_limit;
        tokio::select! {
            _ = ticker.tick() => {
                let msg = AgentMessage::Metrics(collector.sample());
                if let Err(e) = send_message(&mut sink, &msg, idle_limit).await {
                    break Err(e);
                }
            }
            _ = pinger.tick() => {
                if let Err(e) = send_within(&mut sink, Message::Ping(Default::default()), idle_limit).await {
                    break Err(e);
                }
            }
            _ = tokio::time::sleep_until(silent_until) => {
                break Err(anyhow::Error::msg(MASTER_SILENT));
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
                    TaskEvent::Output { task_id, frame, text } => {
                        AgentMessage::TaskOutput { task_id, frame, text }
                    }
                };
                if let Err(e) = send_message(&mut sink, &msg, idle_limit).await {
                    break Err(e);
                }
            }
            incoming = next_master_message(&mut stream, &mut last_heard) => match incoming {
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
                    break Err(anyhow!("master rejected us: {reason}"));
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
    loop {
        reconnect(&config, BACKOFF, None).await;
    }
}

/// Like [`run_forever`], for a master found by discovery: `find` returns
/// the master URL (`ws://host:port/ws/agent`) and replaces
/// `config.master_url`. After [`REDISCOVER_AFTER_FAILURES`] connections in
/// a row fail before the master lets us in, the agent calls `find` again,
/// since the master may have come back at another address. `find` should
/// keep looking until it finds one (see [`find_master_forever`]).
///
/// Reports [`AgentStatus::Searching`] while `find` runs.
pub async fn run_forever_discovering<F, Fut>(config: AgentConfig, find: F) -> !
where
    F: FnMut() -> Fut,
    Fut: Future<Output = String>,
{
    rediscover_forever(config, find, BACKOFF).await
}

async fn rediscover_forever<F, Fut>(mut config: AgentConfig, mut find: F, backoff: Backoff) -> !
where
    F: FnMut() -> Fut,
    Fut: Future<Output = String>,
{
    loop {
        config.report(AgentStatus::Searching);
        config.master_url = find().await;
        reconnect(&config, backoff, Some(REDISCOVER_AFTER_FAILURES)).await;
        info!(
            "{REDISCOVER_AFTER_FAILURES} connections to {} failed in a row; looking for the \
             master again",
            config.master_url
        );
    }
}

/// Runs sessions one after another, waiting `backoff` in between. Returns
/// once `give_up_after` sessions in a row ended before the master let us in;
/// with `None`, never returns.
async fn reconnect(config: &AgentConfig, backoff: Backoff, give_up_after: Option<u32>) {
    let mut delay = backoff.first;
    let mut failures = 0u32;
    config.report(AgentStatus::Connecting {
        master: config.master_url.clone(),
        last_error: None,
    });
    loop {
        let mut welcomed = false;
        let error = match session(config, &mut welcomed).await {
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
            delay = backoff.first;
            failures = 0;
        } else {
            failures = failures.saturating_add(1);
        }
        if give_up_after.is_some_and(|limit| failures >= limit) {
            return;
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(backoff.max);
    }
}

/// Looks for a master on the local network until one answers, logging each
/// miss. Returns its address as `ip:port`.
pub async fn find_master_forever() -> String {
    info!("looking for a master on the local network...");
    let (ip, port) = retry_until_ok(
        || discovery::find_master(Duration::from_secs(30)),
        DISCOVERY_RETRY_PAUSE,
    )
    .await;
    format!("{ip}:{port}")
}

/// Calls `attempt` until it succeeds, logging each failure and waiting
/// `pause` before the next try.
async fn retry_until_ok<T, E, F, Fut>(mut attempt: F, pause: Duration) -> T
where
    E: Display,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    loop {
        match attempt().await {
            Ok(value) => return value,
            Err(e) => {
                info!("no master yet ({e}); still looking");
                tokio::time::sleep(pause).await;
            }
        }
    }
}

/// Sends `msg`, giving up if the master does not take it within `limit`.
async fn send_message<S>(sink: &mut S, msg: &AgentMessage, limit: Duration) -> Result<()>
where
    S: futures_util::Sink<Message, Error = tungstenite::Error> + Unpin,
{
    send_within(sink, Message::text(serde_json::to_string(msg)?), limit).await
}

/// Sends `frame`, giving up if the master does not take it within `limit`
/// (a full send buffer to a master that stopped reading).
async fn send_within<S>(sink: &mut S, frame: Message, limit: Duration) -> Result<()>
where
    S: futures_util::Sink<Message, Error = tungstenite::Error> + Unpin,
{
    match tokio::time::timeout(limit, sink.send(frame)).await {
        Ok(sent) => Ok(sent?),
        Err(_) => Err(anyhow::Error::msg(MASTER_SILENT)),
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
            let (output_tx, mut output_rx) = mpsc::channel::<(u32, String)>(16);
            let forward_outputs = {
                let events = events.clone();
                async move {
                    while let Some((frame, text)) = output_rx.recv().await {
                        let _ = events
                            .send(TaskEvent::Output {
                                task_id,
                                frame,
                                text,
                            })
                            .await;
                    }
                }
            };
            let run = async {
                let result = runner
                    .run_with_outputs(&spec.payload, &progress_tx, &output_tx)
                    .await;
                drop(progress_tx);
                drop(output_tx);
                result
            };
            let (result, (), ()) = tokio::join!(run, forward, forward_outputs);
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

/// Next protocol message; `None` once the master closes the connection.
/// Every frame, pings and pongs included, sets `last_heard` before being
/// looked at, so a quiet but healthy master still counts as alive.
async fn next_master_message<S>(
    stream: &mut S,
    last_heard: &mut Instant,
) -> Result<Option<MasterMessage>>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while let Some(frame) = stream.next().await {
        let frame = frame?;
        *last_heard = Instant::now();
        match frame {
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

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };

    use tokio::net::TcpListener;

    use super::*;

    const fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn keepalive_follows_the_metrics_interval_within_bounds() {
        // Tests report every 50 ms: the floors apply.
        assert_eq!(keepalive(Duration::from_millis(50)), (secs(1), secs(3)));
        assert_eq!(
            keepalive(Duration::from_millis(500)),
            (Duration::from_millis(2500), Duration::from_millis(7500))
        );
        // The command-line and app default of one second.
        assert_eq!(keepalive(secs(1)), (secs(5), secs(15)));
        assert_eq!(keepalive(secs(5)), (secs(5), secs(15)));
        assert_eq!(keepalive(Duration::MAX), (secs(5), secs(15)));
    }

    #[tokio::test]
    async fn retry_until_ok_keeps_trying_until_it_succeeds() {
        let mut calls = 0;
        let found = retry_until_ok(
            || {
                calls += 1;
                let attempt = calls;
                async move {
                    if attempt < 3 {
                        Err("nothing yet")
                    } else {
                        Ok(attempt)
                    }
                }
            },
            Duration::from_millis(1),
        )
        .await;
        assert_eq!(found, 3);
    }

    #[tokio::test]
    async fn looks_for_the_master_again_after_repeated_failures() {
        // The old address: something accepts connections there and drops
        // them at once, so every attempt fails quickly.
        let old = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let old_url = format!("ws://{}/ws/agent", old.local_addr().unwrap());
        let old_hits = Arc::new(AtomicU32::new(0));
        let hits = old_hits.clone();
        let old_server = tokio::spawn(async move {
            loop {
                let (tcp, _) = old.accept().await.unwrap();
                hits.fetch_add(1, Ordering::SeqCst);
                drop(tcp);
            }
        });
        // Where the master is now.
        let new = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let new_url = format!("ws://{}/ws/agent", new.local_addr().unwrap());

        let finds = Arc::new(AtomicU32::new(0));
        let counter = finds.clone();
        let find = move || {
            let url = if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                old_url.clone()
            } else {
                new_url.clone()
            };
            async move { url }
        };
        let config = AgentConfig {
            master_url: String::new(),
            identity: Identity::generate().unwrap(),
            token: None,
            runners: RunnerSet::new(),
            metrics_interval: Duration::from_millis(50),
            status: None,
        };
        let backoff = Backoff {
            first: Duration::from_millis(1),
            max: Duration::from_millis(5),
        };
        let agent = tokio::spawn(async move {
            rediscover_forever(config, find, backoff).await;
        });

        let reached_new = tokio::time::timeout(secs(10), new.accept()).await;
        agent.abort();
        old_server.abort();
        assert!(reached_new.is_ok(), "the agent never tried the new address");
        assert_eq!(old_hits.load(Ordering::SeqCst), REDISCOVER_AFTER_FAILURES);
        assert_eq!(finds.load(Ordering::SeqCst), 2);
    }
}
