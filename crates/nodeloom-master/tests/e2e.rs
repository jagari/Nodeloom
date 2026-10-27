//! End-to-end: a real master, real agents (FakeRunner) and real sockets on
//! loopback. Hand-rolled "raw" agents speak the wire protocol directly to
//! check what an unapproved or misbehaving machine can and cannot do.

use std::{path::Path, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use nodeloom_agent::{AgentConfig, AgentStatus, Identity, run_forever};
use nodeloom_core::{FakeRunner, RunnerSet, identity::DeviceKey};
use nodeloom_master::{Cluster, DeviceStore, Settings, serve};
use nodeloom_proto::{
    AgentMessage, ClusterSnapshot, JobView, MasterMessage, Metrics, NodeId, NodeInfo,
    PROTOCOL_VERSION, RunnerKind, SubmitJob, TaskId, TaskPayload, TaskState,
};
use tokio::{net::TcpListener, sync::watch, task::JoinHandle};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};

const TOKEN: &str = "test-token";

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct Harness {
    base: String,
    ws_base: String,
    tasks: Vec<JoinHandle<()>>,
}

impl Drop for Harness {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
    }
}

async fn start(cluster: Cluster) -> Harness {
    let cluster = Arc::new(cluster);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        serve(listener, cluster).await.unwrap();
    });
    Harness {
        base: format!("http://{addr}"),
        ws_base: format!("ws://{addr}"),
        tasks: vec![server],
    }
}

async fn start_master() -> Harness {
    start(Cluster::new(TOKEN)).await
}

/// A master that keeps paired machines in `dir`, like a real one.
async fn start_master_in(dir: &Path) -> Harness {
    start(Cluster::with_store(TOKEN, DeviceStore::open(dir).unwrap())).await
}

fn identity(id: &str) -> Identity {
    Identity {
        node_id: NodeId(id.into()),
        key: DeviceKey::generate().unwrap(),
    }
}

fn info_of(me: &Identity) -> NodeInfo {
    NodeInfo {
        id: me.node_id.clone(),
        hostname: format!("{}-host", me.node_id),
        os: "test".into(),
        arch: "test".into(),
        cpu_cores: 2,
        total_memory_bytes: 1 << 30,
        runners: vec![RunnerKind::Fake],
    }
}

/// Next protocol message, skipping pings; `None` once the socket closes.
async fn next_message(ws: &mut Ws) -> Option<MasterMessage> {
    while let Some(Ok(frame)) = ws.next().await {
        match frame {
            Message::Text(t) => return Some(serde_json::from_str(t.as_str()).unwrap()),
            Message::Close(_) => return None,
            _ => {}
        }
    }
    None
}

async fn send(ws: &mut Ws, msg: &AgentMessage) {
    ws.send(Message::text(serde_json::to_string(msg).unwrap()))
        .await
        .unwrap();
}

fn rejected_because(reply: Option<MasterMessage>, needle: &str) -> bool {
    matches!(reply, Some(MasterMessage::Rejected { reason }) if reason.contains(needle))
}

impl Harness {
    /// Runs a real agent; returns its index in `tasks` so a test can kill it.
    fn spawn_agent(&mut self, me: Identity, token: Option<&str>) -> usize {
        self.spawn_agent_reporting(me, token, None)
    }

    fn spawn_agent_reporting(
        &mut self,
        me: Identity,
        token: Option<&str>,
        status: Option<watch::Sender<AgentStatus>>,
    ) -> usize {
        let config = AgentConfig {
            master_url: format!("{}/ws/agent", self.ws_base),
            identity: me,
            token: token.map(str::to_owned),
            runners: RunnerSet::new().with(Arc::new(FakeRunner)),
            metrics_interval: Duration::from_millis(50),
            status,
        };
        self.tasks.push(tokio::spawn(async move {
            run_forever(config).await;
        }));
        self.tasks.len() - 1
    }

    /// An agent that joins with the admin token, so needs no approval.
    fn add_agent(&mut self, id: &str, token: &str) -> usize {
        self.spawn_agent(identity(id), Some(token))
    }

    async fn raw_connect(&self) -> Ws {
        tokio_tungstenite::connect_async(format!("{}/ws/agent", self.ws_base))
            .await
            .unwrap()
            .0
    }

    /// Says hello as `me` and returns the challenge nonce.
    async fn raw_hello_only(&self, me: &Identity, token: Option<&str>) -> (Ws, String) {
        let mut ws = self.raw_connect().await;
        let hello = AgentMessage::Hello {
            protocol: PROTOCOL_VERSION,
            public_key: me.key.public_key().as_hex().to_owned(),
            info: info_of(me),
            token: token.map(str::to_owned),
        };
        send(&mut ws, &hello).await;
        match next_message(&mut ws).await {
            Some(MasterMessage::Challenge { nonce }) => (ws, nonce),
            other => panic!("expected a challenge, got {other:?}"),
        }
    }

    /// A complete handshake as `me`; returns the master's decision.
    async fn raw_hello(&self, me: &Identity, token: Option<&str>) -> (Ws, Option<MasterMessage>) {
        let (mut ws, nonce) = self.raw_hello_only(me, token).await;
        let signature = me.key.sign_hello(&nonce, &me.node_id);
        send(&mut ws, &AgentMessage::Proof { signature }).await;
        let reply = next_message(&mut ws).await;
        (ws, reply)
    }

    fn bearer(&self) -> String {
        format!("Bearer {TOKEN}")
    }

    async fn state(&self) -> ClusterSnapshot {
        reqwest::Client::new()
            .get(format!("{}/api/state", self.base))
            .header("Authorization", self.bearer())
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    /// Status of an authorized request to the dashboard API.
    async fn call(&self, method: reqwest::Method, path: &str) -> u16 {
        reqwest::Client::new()
            .request(method, format!("{}{path}", self.base))
            .header("Authorization", self.bearer())
            .send()
            .await
            .unwrap()
            .status()
            .as_u16()
    }

    async fn approve(&self, public_key: &str) -> u16 {
        self.call(
            reqwest::Method::POST,
            &format!("/api/pending/{public_key}/approve"),
        )
        .await
    }

    async fn post_job(&self, job: &SubmitJob) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/api/jobs", self.base))
            .header("Authorization", self.bearer())
            .json(job)
            .send()
            .await
            .unwrap()
    }

    async fn wait_for<F: Fn(&ClusterSnapshot) -> bool>(
        &self,
        what: &str,
        cond: F,
    ) -> ClusterSnapshot {
        for _ in 0..400 {
            let snap = self.state().await;
            if cond(&snap) {
                return snap;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("timed out waiting for: {what}\n{:#?}", self.state().await);
    }
}

fn fake_job(start: u32, end: u32, millis: u64) -> SubmitJob {
    SubmitJob {
        payload: TaskPayload::Fake {
            frame_start: start,
            frame_end: end,
            millis_per_frame: millis,
        },
    }
}

fn all_done(s: &ClusterSnapshot) -> bool {
    s.jobs
        .first()
        .is_some_and(|j| j.tasks.iter().all(|t| t.state == TaskState::Done))
}

#[tokio::test]
async fn job_is_split_run_and_completed_across_three_agents() {
    let mut h = start_master().await;
    for id in ["a", "b", "c"] {
        h.add_agent(id, TOKEN);
    }
    h.wait_for("3 nodes with metrics", |s| {
        s.nodes.len() == 3 && s.nodes.iter().all(|n| n.metrics.is_some())
    })
    .await;

    let job: JobView = h.post_job(&fake_job(1, 30, 5)).await.json().await.unwrap();
    assert_eq!(job.tasks.len(), 3);

    let snap = h.wait_for("all tasks done", all_done).await;

    let tasks = &snap.jobs[0].tasks;
    let mut covered: Vec<u32> = tasks
        .iter()
        .flat_map(|t| t.frame_start..=t.frame_end)
        .collect();
    covered.sort();
    assert_eq!(
        covered,
        (1..=30).collect::<Vec<_>>(),
        "each frame exactly once"
    );
    let mut nodes: Vec<_> = tasks.iter().map(|t| t.node.clone().unwrap().0).collect();
    nodes.sort();
    assert_eq!(nodes, ["a", "b", "c"], "one task per agent");
    assert!(
        tasks
            .iter()
            .all(|t| t.frames_done == t.frame_end - t.frame_start + 1)
    );
}

#[tokio::test]
async fn agent_with_wrong_token_is_rejected_not_queued() {
    let mut h = start_master().await;
    h.add_agent("intruder", "wrong-token");
    h.add_agent("good", TOKEN);
    h.wait_for("good agent registered", |s| !s.nodes.is_empty())
        .await;

    let (_ws, reply) = h.raw_hello(&identity("intruder2"), Some("nope")).await;
    assert!(rejected_because(reply, "wrong token"));
    let snap = h.state().await;
    assert_eq!(snap.nodes.len(), 1);
    assert_eq!(snap.nodes[0].info.id.0, "good");
    assert!(snap.pending.is_empty(), "a wrong token is not a request");
}

#[tokio::test]
async fn unapproved_machine_gets_no_work_and_no_cluster_info() {
    let mut h = start_master().await;
    let lurker = identity("lurker");
    let (mut ws, reply) = h.raw_hello(&lurker, None).await;
    assert_eq!(reply, Some(MasterMessage::Pending));

    let snap = h.state().await;
    assert!(snap.nodes.is_empty());
    assert_eq!(snap.pending.len(), 1);
    let request = &snap.pending[0];
    assert_eq!(request.code, lurker.confirmation_code());
    assert_eq!(request.info.id, lurker.node_id);
    assert_eq!(request.address.as_deref(), Some("127.0.0.1"));

    // Nothing can run on a machine that is only waiting.
    assert_eq!(h.post_job(&fake_job(1, 4, 0)).await.status().as_u16(), 409);

    // Reports from a pending machine are dropped unread.
    send(
        &mut ws,
        &AgentMessage::TaskFailed {
            task_id: TaskId(0),
            error: "forged".into(),
        },
    )
    .await;
    send(&mut ws, &AgentMessage::Metrics(Metrics::default())).await;

    h.add_agent("worker", TOKEN);
    h.wait_for("worker joined", |s| s.nodes.len() == 1).await;
    assert_eq!(h.post_job(&fake_job(1, 10, 1)).await.status().as_u16(), 200);
    let snap = h.wait_for("job done on the worker", all_done).await;
    assert!(
        snap.jobs[0]
            .tasks
            .iter()
            .all(|t| t.node.as_ref().unwrap().0 == "worker")
    );
    assert_eq!(snap.pending.len(), 1, "still waiting, still connected");

    // The job is finished, so any task meant for the lurker would have been
    // sent by now. Its socket holds nothing but pings.
    let quiet = tokio::time::timeout(Duration::from_millis(300), next_message(&mut ws)).await;
    assert!(quiet.is_err(), "pending machine was sent {quiet:?}");
}

#[tokio::test]
async fn approved_machine_reconnects_without_asking_again() {
    let mut h = start_master().await;
    let me = identity("laptop");
    let agent = h.spawn_agent(me.clone(), None);
    let snap = h
        .wait_for("request appears", |s| s.pending.len() == 1)
        .await;
    assert_eq!(snap.pending[0].code, me.confirmation_code());
    assert_eq!(h.approve(&snap.pending[0].public_key).await, 204);
    let snap = h
        .wait_for("approved agent joins", |s| s.nodes.len() == 1)
        .await;
    assert!(snap.pending.is_empty());
    assert_eq!(snap.devices.len(), 1);
    assert_eq!(snap.devices[0].id, me.node_id);

    h.tasks[agent].abort();
    h.wait_for("agent gone", |s| s.nodes.is_empty()).await;
    h.spawn_agent(me, None);
    let snap = h.wait_for("agent back", |s| s.nodes.len() == 1).await;
    assert!(snap.pending.is_empty(), "no second approval needed");
}

#[tokio::test]
async fn agent_status_follows_pairing_and_removal() {
    let mut h = start_master().await;
    let me = identity("desk");
    let master = format!("{}/ws/agent", h.ws_base);
    let (tx, mut rx) = watch::channel(AgentStatus::Searching);
    h.spawn_agent_reporting(me.clone(), None, Some(tx));

    let expected = AgentStatus::WaitingForApproval {
        master: master.clone(),
        code: me.confirmation_code(),
    };
    wait_status(&mut rx, "waiting with the code", |s| *s == expected).await;
    let snap = h
        .wait_for("request appears", |s| s.pending.len() == 1)
        .await;
    assert_eq!(h.approve(&snap.pending[0].public_key).await, 204);
    wait_status(&mut rx, "joined", |s| {
        *s == AgentStatus::Joined {
            master: master.clone(),
        }
    })
    .await;

    assert_eq!(
        h.call(reqwest::Method::DELETE, "/api/devices/desk").await,
        204
    );
    // The reason stays visible while the agent waits to retry.
    wait_status(&mut rx, "told it was removed", |s| {
        matches!(s, AgentStatus::Connecting { last_error: Some(e), .. } if e.contains("removed"))
    })
    .await;
}

async fn wait_status(
    rx: &mut watch::Receiver<AgentStatus>,
    what: &str,
    cond: impl FnMut(&AgentStatus) -> bool,
) {
    let reached = tokio::time::timeout(Duration::from_secs(10), rx.wait_for(cond))
        .await
        .is_ok_and(|r| r.is_ok());
    assert!(
        reached,
        "timed out waiting for agent status: {what}; last {:?}",
        *rx.borrow()
    );
}

#[tokio::test]
async fn approvals_survive_a_master_restart() {
    let dir = tempfile::tempdir().unwrap();
    let me = identity("pi");
    {
        let mut first = start_master_in(dir.path()).await;
        first.spawn_agent(me.clone(), None);
        let snap = first
            .wait_for("request appears", |s| s.pending.len() == 1)
            .await;
        assert_eq!(first.approve(&snap.pending[0].public_key).await, 204);
        first.wait_for("approved", |s| s.nodes.len() == 1).await;
    }

    // A new master over the same data directory.
    let mut second = start_master_in(dir.path()).await;
    assert_eq!(second.state().await.devices.len(), 1);
    second.spawn_agent(me, None);
    let snap = second.wait_for("rejoined", |s| s.nodes.len() == 1).await;
    assert!(snap.pending.is_empty());
}

#[tokio::test]
async fn removed_machine_is_cut_off_and_stays_out() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = start_master_in(dir.path()).await;
    let victim = identity("a");
    h.spawn_agent(victim.clone(), Some(TOKEN));
    h.add_agent("b", TOKEN);
    h.wait_for("2 nodes", |s| s.nodes.len() == 2).await;
    h.post_job(&fake_job(1, 20, 100)).await;
    h.wait_for("both tasks running", |s| {
        s.jobs
            .first()
            .is_some_and(|j| j.tasks.iter().all(|t| t.state == TaskState::Running))
    })
    .await;

    assert_eq!(h.call(reqwest::Method::DELETE, "/api/devices/a").await, 204);
    let snap = h
        .wait_for("job finished on b alone", |s| {
            s.nodes.len() == 1 && all_done(s)
        })
        .await;
    assert!(
        snap.jobs[0]
            .tasks
            .iter()
            .all(|t| t.node.as_ref().unwrap().0 == "b")
    );
    assert_eq!(snap.devices.len(), 1);

    // The key is dead, token or not, and it does not even get queued.
    let (_ws, reply) = h.raw_hello(&victim, Some(TOKEN)).await;
    assert!(rejected_because(reply, "removed"));
    let (_ws, reply) = h.raw_hello(&victim, None).await;
    assert!(rejected_because(reply, "removed"));
    assert!(h.state().await.pending.is_empty());
    assert_eq!(h.call(reqwest::Method::DELETE, "/api/devices/a").await, 404);

    // Still dead after a restart.
    let restarted = start_master_in(dir.path()).await;
    let (_ws, reply) = restarted.raw_hello(&victim, None).await;
    assert!(rejected_because(reply, "removed"));

    // A fresh identity (reset-identity) is a new machine with a new code.
    let fresh = identity("a2");
    let (_ws, reply) = restarted.raw_hello(&fresh, None).await;
    assert_eq!(reply, Some(MasterMessage::Pending));
    let snap = restarted.state().await;
    assert_eq!(snap.pending[0].code, fresh.confirmation_code());
    assert_ne!(fresh.confirmation_code(), victim.confirmation_code());
}

#[tokio::test]
async fn ignored_machine_is_refused_until_the_master_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let h = start_master_in(dir.path()).await;
    let me = identity("stranger");
    let (mut ws, reply) = h.raw_hello(&me, None).await;
    assert_eq!(reply, Some(MasterMessage::Pending));

    let key = me.key.public_key().as_hex().to_owned();
    let ignore = format!("/api/pending/{key}/ignore");
    assert_eq!(h.call(reqwest::Method::POST, &ignore).await, 204);
    assert!(rejected_because(next_message(&mut ws).await, "ignored"));
    let (_ws, reply) = h.raw_hello(&me, None).await;
    assert!(rejected_because(reply, "ignored"));
    assert!(h.state().await.pending.is_empty());
    assert_eq!(h.call(reqwest::Method::POST, &ignore).await, 404);

    let restarted = start_master_in(dir.path()).await;
    let (_ws, reply) = restarted.raw_hello(&me, None).await;
    assert_eq!(reply, Some(MasterMessage::Pending));
}

#[tokio::test]
async fn paired_id_with_another_key_is_rejected_and_does_not_displace_it() {
    let mut h = start_master().await;
    h.add_agent("a", TOKEN);
    h.wait_for("a joined", |s| s.nodes.len() == 1).await;

    let impostor = identity("a");
    let (_ws, reply) = h.raw_hello(&impostor, None).await;
    assert!(rejected_because(reply, "already paired"));
    let (_ws, reply) = h.raw_hello(&impostor, Some(TOKEN)).await;
    assert!(rejected_because(reply, "already paired"));

    let snap = h.state().await;
    assert_eq!(snap.nodes.len(), 1, "the real machine stays connected");
    assert!(snap.pending.is_empty());
}

#[tokio::test]
async fn token_approval_clears_other_requests_for_the_same_id() {
    let h = start_master().await;
    let waiting = identity("x");
    let (mut ws, reply) = h.raw_hello(&waiting, None).await;
    assert_eq!(reply, Some(MasterMessage::Pending));

    let (_ws2, reply) = h.raw_hello(&identity("x"), Some(TOKEN)).await;
    assert_eq!(reply, Some(MasterMessage::Welcome));
    assert!(rejected_because(
        next_message(&mut ws).await,
        "another machine was approved"
    ));
    let key = waiting.key.public_key().as_hex().to_owned();
    assert_eq!(h.approve(&key).await, 404);
}

#[tokio::test]
async fn a_recorded_proof_cannot_be_replayed() {
    let h = start_master().await;
    let me = identity("a");
    let (mut first, nonce) = h.raw_hello_only(&me, None).await;
    let signature = me.key.sign_hello(&nonce, &me.node_id);
    send(
        &mut first,
        &AgentMessage::Proof {
            signature: signature.clone(),
        },
    )
    .await;
    assert_eq!(next_message(&mut first).await, Some(MasterMessage::Pending));

    // Someone who saw that exchange says hello with the same key and
    // replays the signature: the new challenge does not match.
    let (mut second, _) = h.raw_hello_only(&me, None).await;
    send(&mut second, &AgentMessage::Proof { signature }).await;
    assert!(rejected_because(
        next_message(&mut second).await,
        "signature"
    ));
}

#[tokio::test]
async fn agents_of_another_version_get_a_readable_answer() {
    let h = start_master().await;
    let mut ws = h.raw_connect().await;
    // Shaped like a version 1 hello: a shared token and no key.
    let old = serde_json::json!({
        "type": "hello",
        "protocol": 1,
        "token": TOKEN,
        "info": info_of(&identity("old")),
    });
    ws.send(Message::text(old.to_string())).await.unwrap();
    assert!(rejected_because(
        next_message(&mut ws).await,
        "protocol 1 is not supported"
    ));
}

#[tokio::test]
async fn invalid_self_descriptions_are_refused_before_the_challenge() {
    let h = start_master().await;
    let me = identity("a");
    let mut info = info_of(&me);
    info.hostname = "MacBook\u{202E}".into();
    let mut ws = h.raw_connect().await;
    send(
        &mut ws,
        &AgentMessage::Hello {
            protocol: PROTOCOL_VERSION,
            public_key: me.key.public_key().as_hex().to_owned(),
            info,
            token: None,
        },
    )
    .await;
    assert!(rejected_because(next_message(&mut ws).await, "hostname"));
}

#[tokio::test]
async fn browser_pages_cannot_open_the_agent_socket() {
    let h = start_master().await;
    let mut request = format!("{}/ws/agent", h.ws_base)
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("Origin", "http://example.com".parse().unwrap());
    match tokio_tungstenite::connect_async(request).await {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status().as_u16(), 403)
        }
        other => panic!("expected 403, got {:?}", other.map(|_| ())),
    }
}

#[tokio::test]
async fn silent_pending_connections_are_dropped() {
    // Generous margins: a live peer answers within a few milliseconds, but
    // CI machines can stall for a few hundred.
    let cluster = Cluster::new(TOKEN).with_settings(Settings {
        ping_interval: Duration::from_millis(100),
        pending_idle_timeout: Duration::from_secs(1),
        ..Settings::default()
    });
    let h = start(cluster).await;

    // Never reads again, so never answers a ping.
    let (_dead, reply) = h.raw_hello(&identity("dead"), None).await;
    assert_eq!(reply, Some(MasterMessage::Pending));
    // Keeps reading, so tungstenite answers every ping.
    let (mut alive, _) = h.raw_hello(&identity("alive"), None).await;
    let reader = tokio::spawn(async move { while next_message(&mut alive).await.is_some() {} });

    let snap = h
        .wait_for("dead request dropped", |s| {
            s.pending.iter().all(|p| p.info.id.0 != "dead")
        })
        .await;
    assert_eq!(snap.pending.len(), 1);
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert_eq!(h.state().await.pending.len(), 1, "live request kept");
    reader.abort();
}

#[tokio::test]
async fn ui_endpoints_require_the_token() {
    let h = start_master().await;
    let client = reqwest::Client::new();
    let status = |r: reqwest::Response| r.status().as_u16();

    let state = format!("{}/api/state", h.base);
    assert_eq!(status(client.get(&state).send().await.unwrap()), 401);
    assert_eq!(
        status(
            client
                .get(&state)
                .header("Authorization", "Bearer nope")
                .send()
                .await
                .unwrap()
        ),
        401
    );
    // The query-string form is for browser WebSockets only.
    assert_eq!(
        status(
            client
                .get(format!("{state}?token={TOKEN}"))
                .send()
                .await
                .unwrap()
        ),
        401
    );
    assert_eq!(
        status(
            client
                .post(format!("{}/api/jobs", h.base))
                .json(&fake_job(1, 2, 0))
                .send()
                .await
                .unwrap()
        ),
        401
    );
    for (method, path) in [
        (reqwest::Method::POST, "/api/pending/abc/approve"),
        (reqwest::Method::POST, "/api/pending/abc/ignore"),
        (reqwest::Method::DELETE, "/api/devices/abc"),
    ] {
        let url = format!("{}{path}", h.base);
        assert_eq!(
            status(client.request(method, url).send().await.unwrap()),
            401,
            "{path}"
        );
    }
    assert_eq!(h.approve("abc").await, 404);
    assert_eq!(
        h.call(reqwest::Method::DELETE, "/api/devices/abc").await,
        404
    );
}

#[tokio::test]
async fn submit_errors_map_to_http_statuses() {
    let mut h = start_master().await;
    assert_eq!(
        h.post_job(&fake_job(1, 5, 0)).await.status().as_u16(),
        409,
        "no nodes connected"
    );
    h.add_agent("a", TOKEN);
    h.wait_for("agent registered", |s| !s.nodes.is_empty())
        .await;
    assert_eq!(
        h.post_job(&fake_job(9, 1, 0)).await.status().as_u16(),
        400,
        "reversed range"
    );
    let escape = SubmitJob {
        payload: TaskPayload::Blender {
            blend_file: "../../etc/passwd.blend".into(),
            frame_start: 1,
            frame_end: 2,
        },
    };
    assert_eq!(
        h.post_job(&escape).await.status().as_u16(),
        400,
        "path traversal"
    );
}

#[tokio::test]
async fn ui_websocket_streams_snapshots() {
    let mut h = start_master().await;
    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("{}/ws/ui?token={TOKEN}", h.ws_base))
            .await
            .unwrap();

    let first: ClusterSnapshot = match ws.next().await.unwrap().unwrap() {
        Message::Text(t) => serde_json::from_str(t.as_str()).unwrap(),
        other => panic!("unexpected frame {other:?}"),
    };
    assert!(first.nodes.is_empty());

    h.add_agent("a", TOKEN);
    let saw_agent = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(frame)) = ws.next().await {
            if let Message::Text(t) = frame {
                let snap: ClusterSnapshot = serde_json::from_str(t.as_str()).unwrap();
                if snap.nodes.len() == 1 {
                    return true;
                }
            }
        }
        false
    })
    .await
    .unwrap();
    assert!(saw_agent);

    // A bad token must not upgrade.
    let err = tokio_tungstenite::connect_async(format!("{}/ws/ui?token=bad", h.ws_base)).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn work_moves_to_another_agent_when_one_disappears() {
    let mut h = start_master().await;
    let victim = h.add_agent("a", TOKEN);
    h.add_agent("b", TOKEN);
    h.wait_for("2 nodes", |s| s.nodes.len() == 2).await;

    // Long enough that the job is still running when we kill agent "a".
    h.post_job(&fake_job(1, 20, 100)).await;
    h.wait_for("both tasks running", |s| {
        s.jobs
            .first()
            .is_some_and(|j| j.tasks.iter().all(|t| t.state == TaskState::Running))
    })
    .await;

    h.tasks[victim].abort();

    let snap = h
        .wait_for("job finished on the survivor", |s| {
            s.nodes.len() == 1 && all_done(s)
        })
        .await;
    assert!(
        snap.jobs[0]
            .tasks
            .iter()
            .all(|t| t.node.as_ref().unwrap().0 == "b")
    );
}

#[tokio::test]
async fn cors_preflight_allows_authorized_ui_requests() {
    let h = start_master().await;
    for (method, path) in [("POST", "/api/jobs"), ("DELETE", "/api/devices/x")] {
        let response = reqwest::Client::new()
            .request(reqwest::Method::OPTIONS, format!("{}{path}", h.base))
            .header("Origin", "http://localhost:5173")
            .header("Access-Control-Request-Method", method)
            .header(
                "Access-Control-Request-Headers",
                "authorization,content-type",
            )
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        let headers = response.headers();
        assert_eq!(headers["access-control-allow-origin"], "*");
        let allowed = headers["access-control-allow-headers"]
            .to_str()
            .unwrap()
            .to_lowercase();
        assert!(allowed.contains("authorization") && allowed.contains("content-type"));
        let methods = headers["access-control-allow-methods"]
            .to_str()
            .unwrap()
            .to_uppercase();
        assert!(methods.contains(method), "{methods}");
    }
}

#[tokio::test]
async fn dashboard_page_is_served_with_protective_headers() {
    let h = start_master().await;
    let page = reqwest::get(format!("{}/", h.base)).await.unwrap();
    assert_eq!(page.status().as_u16(), 200);
    let headers = page.headers().clone();
    assert!(
        headers["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert_eq!(headers["x-content-type-options"], "nosniff");
    let csp = headers["content-security-policy"].to_str().unwrap();
    assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    // The page itself needs no token; everything it shows comes from the API.
    assert!(page.text().await.unwrap().contains("Nodeloom"));

    // Unknown API paths stay errors instead of falling through to the page.
    for path in ["/api/nope", "/ws/nope"] {
        let status = reqwest::get(format!("{}{path}", h.base))
            .await
            .unwrap()
            .status();
        assert_eq!(status.as_u16(), 404, "{path}");
    }
}

#[cfg(feature = "embed-ui")]
#[tokio::test]
async fn bundled_dashboard_assets_are_served() {
    let h = start_master().await;
    let html = reqwest::get(format!("{}/", h.base))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    // Every script and stylesheet the page names must come back from the
    // master, cacheable for good (Vite hashes their names).
    let assets: Vec<&str> = html
        .split('"')
        .filter(|s| s.starts_with("/assets/"))
        .collect();
    assert!(!assets.is_empty(), "{html}");
    for asset in assets {
        let response = reqwest::get(format!("{}{asset}", h.base)).await.unwrap();
        assert_eq!(response.status().as_u16(), 200, "{asset}");
        assert!(
            response.headers()["cache-control"]
                .to_str()
                .unwrap()
                .contains("immutable")
        );
    }
}
