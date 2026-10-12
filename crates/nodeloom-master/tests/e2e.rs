//! End-to-end: a real master, real agents (FakeRunner) and real sockets on
//! loopback.

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use nodeloom_agent::{AgentConfig, run_forever};
use nodeloom_core::{FakeRunner, RunnerSet};
use nodeloom_master::{Cluster, serve};
use nodeloom_proto::{ClusterSnapshot, JobView, NodeId, SubmitJob, TaskPayload, TaskState};
use tokio::{net::TcpListener, task::JoinHandle};

const TOKEN: &str = "test-token";

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

async fn start_master() -> Harness {
    let cluster = Arc::new(Cluster::new(TOKEN));
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

impl Harness {
    fn add_agent(&mut self, id: &str, token: &str) {
        let config = AgentConfig {
            master_url: format!("{}/ws/agent", self.ws_base),
            token: token.into(),
            node_id: NodeId(id.into()),
            runners: RunnerSet::new().with(Arc::new(FakeRunner)),
            metrics_interval: Duration::from_millis(50),
        };
        self.tasks.push(tokio::spawn(async move {
            run_forever(config).await;
        }));
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

    async fn wait_for<F: Fn(&ClusterSnapshot) -> bool>(
        &self,
        what: &str,
        cond: F,
    ) -> ClusterSnapshot {
        for _ in 0..200 {
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

    let job: JobView = reqwest::Client::new()
        .post(format!("{}/api/jobs", h.base))
        .header("Authorization", h.bearer())
        .json(&fake_job(1, 30, 5))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(job.tasks.len(), 3);

    let snap = h
        .wait_for("all tasks done", |s| {
            s.jobs
                .first()
                .is_some_and(|j| j.tasks.iter().all(|t| t.state == TaskState::Done))
        })
        .await;

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
async fn agent_with_wrong_token_is_not_registered() {
    let mut h = start_master().await;
    h.add_agent("intruder", "wrong-token");
    h.add_agent("good", TOKEN);
    let snap = h
        .wait_for("good agent registered", |s| !s.nodes.is_empty())
        .await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let snap2 = h.state().await;
    assert_eq!(snap.nodes.len(), 1);
    assert_eq!(snap2.nodes.len(), 1);
    assert_eq!(snap2.nodes[0].info.id.0, "good");
}

#[tokio::test]
async fn ui_endpoints_require_the_token() {
    let h = start_master().await;
    let client = reqwest::Client::new();

    let status = |r: reqwest::Response| r.status().as_u16();
    assert_eq!(
        status(
            client
                .get(format!("{}/api/state", h.base))
                .send()
                .await
                .unwrap()
        ),
        401
    );
    assert_eq!(
        status(
            client
                .get(format!("{}/api/state", h.base))
                .header("Authorization", "Bearer nope")
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
    // Query-string token works (needed by browser WebSockets).
    assert_eq!(
        status(
            client
                .get(format!("{}/api/state?token={TOKEN}", h.base))
                .send()
                .await
                .unwrap()
        ),
        200
    );
}

#[tokio::test]
async fn submit_errors_map_to_http_statuses() {
    let mut h = start_master().await;
    let client = reqwest::Client::new();
    let url = format!("{}/api/jobs", h.base);
    let auth = h.bearer();
    let post = |job: SubmitJob| {
        let (client, url, auth) = (client.clone(), url.clone(), auth.clone());
        async move {
            client
                .post(url)
                .header("Authorization", auth)
                .json(&job)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16()
        }
    };

    assert_eq!(post(fake_job(1, 5, 0)).await, 409, "no nodes connected");
    h.add_agent("a", TOKEN);
    h.wait_for("agent registered", |s| !s.nodes.is_empty())
        .await;
    assert_eq!(post(fake_job(9, 1, 0)).await, 400, "reversed range");
    let escape = SubmitJob {
        payload: TaskPayload::Blender {
            blend_file: "../../etc/passwd.blend".into(),
            frame_start: 1,
            frame_end: 2,
        },
    };
    assert_eq!(post(escape).await, 400, "path traversal");
}

#[tokio::test]
async fn ui_websocket_streams_snapshots() {
    let mut h = start_master().await;
    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("{}/ws/ui?token={TOKEN}", h.ws_base))
            .await
            .unwrap();

    let first: ClusterSnapshot = match ws.next().await.unwrap().unwrap() {
        tokio_tungstenite::tungstenite::Message::Text(t) => {
            serde_json::from_str(t.as_str()).unwrap()
        }
        other => panic!("unexpected frame {other:?}"),
    };
    assert!(first.nodes.is_empty());

    h.add_agent("a", TOKEN);
    let saw_agent = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(frame)) = ws.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(t) = frame {
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
    h.add_agent("a", TOKEN);
    let victim = h.tasks.len() - 1;
    h.add_agent("b", TOKEN);
    h.wait_for("2 nodes", |s| s.nodes.len() == 2).await;

    // Long enough that the job is still running when we kill agent "a".
    reqwest::Client::new()
        .post(format!("{}/api/jobs", h.base))
        .header("Authorization", h.bearer())
        .json(&fake_job(1, 20, 100))
        .send()
        .await
        .unwrap();
    h.wait_for("both tasks running", |s| {
        s.jobs
            .first()
            .is_some_and(|j| j.tasks.iter().all(|t| t.state == TaskState::Running))
    })
    .await;

    h.tasks[victim].abort();

    let snap = h
        .wait_for("job finished on the survivor", |s| {
            s.nodes.len() == 1
                && s.jobs
                    .first()
                    .is_some_and(|j| j.tasks.iter().all(|t| t.state == TaskState::Done))
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
    let response = reqwest::Client::new()
        .request(reqwest::Method::OPTIONS, format!("{}/api/jobs", h.base))
        .header("Origin", "http://localhost:5173")
        .header("Access-Control-Request-Method", "POST")
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
}
