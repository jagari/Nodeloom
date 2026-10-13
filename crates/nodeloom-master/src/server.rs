//! HTTP and WebSocket endpoints.
//!
//! * `GET    /api/state`                       full [`ClusterSnapshot`]
//! * `POST   /api/jobs`                        submit a [`SubmitJob`]
//! * `POST   /api/pending/{public_key}/approve` let a waiting machine in
//! * `POST   /api/pending/{public_key}/ignore`  refuse it until restart
//! * `DELETE /api/devices/{node_id}`            remove a paired machine for good
//! * `GET    /ws/ui`                           snapshot stream for UI clients
//! * `GET    /ws/agent`                        agent connections
//!
//! UI endpoints need the admin token as `Authorization: Bearer <token>`.
//! `/ws/ui` also takes `?token=<token>` because browsers' WebSocket API
//! cannot set headers. Agents authenticate with their own key instead (see
//! [`handshake`]). Traffic is plain HTTP/WS for now; do not expose the port
//! beyond a trusted network.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{
        ConnectInfo, DefaultBodyLimit, Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        HeaderMap, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE, ORIGIN},
    },
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use nodeloom_core::{
    auth::tokens_match,
    identity::{PublicKey, random_hex},
    validate::validate_node_info,
};
use nodeloom_proto::{
    AgentMessage, ClusterSnapshot, JobView, MasterMessage, NodeId, NodeInfo, PROTOCOL_VERSION,
    SubmitJob,
};
use serde::Deserialize;
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
    time::Instant,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use crate::state::{Admission, Cluster, Decision, PairError, SubmitError};

/// Time an agent has to say hello and answer the challenge.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_WS_MESSAGE: usize = 1 << 20;
/// Handshake messages are tiny. Anything bigger comes from something that is
/// not a well-behaved agent, and is refused before it is parsed.
const MAX_HANDSHAKE_MESSAGE: usize = 8 << 10;
const UI_PUSH_DELAY: Duration = Duration::from_millis(100);

pub fn router(cluster: Arc<Cluster>) -> Router {
    Router::new()
        .route("/api/state", get(get_state))
        .route("/api/jobs", post(post_job))
        .route("/api/pending/{public_key}/approve", post(approve_pending))
        .route("/api/pending/{public_key}/ignore", post(ignore_pending))
        .route("/api/devices/{node_id}", delete(remove_device))
        .route("/ws/ui", get(ws_ui))
        .route("/ws/agent", get(ws_agent))
        .layer(DefaultBodyLimit::max(64 * 1024))
        // UIs run on other origins (Tauri webview, dev server). Auth is a
        // bearer token rather than cookies, so any origin may call the API.
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([AUTHORIZATION, CONTENT_TYPE]),
        )
        .with_state(cluster)
}

pub async fn serve(listener: TcpListener, cluster: Arc<Cluster>) -> std::io::Result<()> {
    // Connection info gives the dashboard the address of a waiting machine.
    axum::serve(
        listener,
        router(cluster).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

fn authorized(cluster: &Cluster, provided: Option<&str>) -> bool {
    provided.is_some_and(|p| tokens_match(cluster.admin_token(), p))
}

type ApiError = (StatusCode, String);

fn unauthorized() -> ApiError {
    (StatusCode::UNAUTHORIZED, String::new())
}

async fn get_state(
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
) -> Result<Json<ClusterSnapshot>, StatusCode> {
    if !authorized(&cluster, bearer(&headers)) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(cluster.snapshot()))
}

async fn post_job(
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    Json(request): Json<SubmitJob>,
) -> Result<Json<JobView>, ApiError> {
    if !authorized(&cluster, bearer(&headers)) {
        return Err(unauthorized());
    }
    cluster.submit_job(request.payload).map(Json).map_err(|e| {
        let status = match e {
            SubmitError::Invalid(_) => StatusCode::BAD_REQUEST,
            SubmitError::NoCapableNodes(_) => StatusCode::CONFLICT,
        };
        (status, e.to_string())
    })
}

fn pair_result(result: Result<Result<(), PairError>, tokio::task::JoinError>) -> Response {
    let error = match result {
        Ok(Ok(())) => return StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => e,
        Err(e) => PairError::Storage(e.to_string()),
    };
    let status = match error {
        PairError::NotFound => StatusCode::NOT_FOUND,
        PairError::Conflict(_) => StatusCode::CONFLICT,
        PairError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, error.to_string()).into_response()
}

async fn approve_pending(
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    Path(public_key): Path<String>,
) -> Response {
    if !authorized(&cluster, bearer(&headers)) {
        return unauthorized().into_response();
    }
    // Approving writes the device list to disk.
    pair_result(tokio::task::spawn_blocking(move || cluster.approve(&public_key)).await)
}

async fn ignore_pending(
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    Path(public_key): Path<String>,
) -> Response {
    if !authorized(&cluster, bearer(&headers)) {
        return unauthorized().into_response();
    }
    pair_result(Ok(cluster.ignore(&public_key)))
}

async fn remove_device(
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    Path(node_id): Path<String>,
) -> Response {
    if !authorized(&cluster, bearer(&headers)) {
        return unauthorized().into_response();
    }
    pair_result(tokio::task::spawn_blocking(move || cluster.remove_device(&NodeId(node_id))).await)
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

async fn ws_ui(
    ws: WebSocketUpgrade,
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Response {
    if !authorized(&cluster, bearer(&headers).or(query.token.as_deref())) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.max_message_size(MAX_WS_MESSAGE)
        .on_upgrade(move |socket| ui_session(socket, cluster))
}

/// Pushes a snapshot on connect and after every change (at most ~10/s).
async fn ui_session(mut socket: WebSocket, cluster: Arc<Cluster>) {
    let mut changes = cluster.subscribe();
    while let Ok(json) = serde_json::to_string(&cluster.snapshot()) {
        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
        tokio::select! {
            changed = changes.recv() => {
                if matches!(changed, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                    break;
                }
                tokio::time::sleep(UI_PUSH_DELAY).await;
                while changes.try_recv().is_ok() {}
            }
            incoming = socket.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => {}
            },
        }
    }
}

async fn ws_agent(
    ws: WebSocketUpgrade,
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Response {
    // Browsers always send Origin on WebSocket handshakes and do not apply
    // CORS to them; agents never send it. Refusing it keeps web pages the
    // user happens to open from filling the pending list.
    if headers.contains_key(ORIGIN) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let address = connect_info.map(|Extension(ConnectInfo(addr))| addr.ip());
    ws.max_message_size(MAX_WS_MESSAGE)
        .on_upgrade(move |socket| agent_session(socket, cluster, address))
}

async fn send_master(socket: &mut WebSocket, msg: &MasterMessage) -> Result<(), axum::Error> {
    let json = serde_json::to_string(msg).expect("MasterMessage serializes");
    socket.send(Message::Text(json.into())).await
}

async fn reject(socket: &mut WebSocket, reason: String) {
    let _ = send_master(socket, &MasterMessage::Rejected { reason }).await;
}

async fn agent_session(
    mut socket: WebSocket,
    cluster: Arc<Cluster>,
    address: Option<std::net::IpAddr>,
) {
    let Ok(Some(hello)) = tokio::time::timeout(HELLO_TIMEOUT, handshake(&mut socket)).await else {
        return;
    };
    let node_id: NodeId = hello.info.id.clone();
    let key = hello.public_key.as_hex().to_owned();
    let from = address.map_or_else(|| "an unknown address".to_owned(), |a| a.to_string());

    match cluster.classify(
        &hello.info,
        &hello.public_key,
        hello.token.as_deref(),
        address,
    ) {
        Admission::Approved => {}
        Admission::ApproveWithToken => {
            let approval = {
                let (cluster, info, key) = (cluster.clone(), hello.info.clone(), key.clone());
                tokio::task::spawn_blocking(move || cluster.approve_with_token(&info, &key)).await
            };
            match approval {
                Ok(Ok(())) => info!("approved {node_id} ({}) by token", hello.info.hostname),
                Ok(Err(e)) => return reject(&mut socket, e.to_string()).await,
                Err(e) => return reject(&mut socket, e.to_string()).await,
            }
        }
        Admission::Rejected(reason) => {
            warn!("refused {node_id} from {from}: {reason}");
            return reject(&mut socket, reason).await;
        }
        Admission::Pending(ticket, decision) => {
            info!(
                "{node_id} ({}) from {from} is waiting for approval",
                hello.info.hostname
            );
            let approved = wait_for_decision(&mut socket, &cluster, decision).await;
            drop(ticket);
            if !approved {
                return;
            }
        }
    }

    let (tx, mut outbox) = mpsc::unbounded_channel();
    let (kick, mut kicked) = oneshot::channel();
    let generation = match cluster.admit(hello.info.clone(), &key, tx, kick) {
        Ok(generation) => generation,
        Err(reason) => return reject(&mut socket, reason).await,
    };
    // Welcome goes out before anything dispatch already queued in `outbox`.
    if send_master(&mut socket, &MasterMessage::Welcome)
        .await
        .is_ok()
    {
        info!(
            "agent {} ({}, {}) connected",
            node_id, hello.info.hostname, hello.info.arch
        );
        loop {
            tokio::select! {
                biased;
                reason = &mut kicked => {
                    // An error means a newer connection replaced this one.
                    if let Ok(reason) = reason {
                        reject(&mut socket, reason).await;
                    }
                    break;
                }
                incoming = socket.recv() => match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<AgentMessage>(text.as_str()) {
                            Ok(msg) => handle_agent_message(&cluster, &node_id, msg),
                            Err(e) => {
                                warn!("agent {node_id} sent an invalid message: {e}");
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                },
                outgoing = outbox.recv() => match outgoing {
                    Some(msg) => {
                        if send_master(&mut socket, &msg).await.is_err() {
                            break;
                        }
                    }
                    // Our registration was replaced by a newer connection.
                    None => break,
                },
            }
        }
    }

    cluster.unregister_node(&node_id, generation);
    info!("agent {node_id} disconnected");
}

/// Holds a pending connection open until someone approves or refuses the
/// machine, or it goes away. Nothing the machine sends is acted on.
/// Returns whether it was approved.
async fn wait_for_decision(
    socket: &mut WebSocket,
    cluster: &Cluster,
    mut decision: oneshot::Receiver<Decision>,
) -> bool {
    if send_master(socket, &MasterMessage::Pending).await.is_err() {
        return false;
    }
    let settings = cluster.settings();
    let mut ping = tokio::time::interval(settings.ping_interval);
    ping.tick().await;
    let mut last_heard = Instant::now();
    loop {
        tokio::select! {
            biased;
            decided = &mut decision => return match decided {
                Ok(Decision::Approved) => true,
                Ok(Decision::Rejected(reason)) => {
                    reject(socket, reason).await;
                    false
                }
                Err(_) => false,
            },
            incoming = socket.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return false,
                // Pongs and anything else only prove the peer is alive.
                Some(Ok(_)) => last_heard = Instant::now(),
            },
            _ = ping.tick() => {
                if last_heard.elapsed() >= settings.pending_idle_timeout
                    || socket.send(Message::Ping(Bytes::new())).await.is_err()
                {
                    return false;
                }
            }
        }
    }
}

/// A machine that proved it holds the private half of `public_key`.
struct Hello {
    info: NodeInfo,
    public_key: PublicKey,
    token: Option<String>,
}

/// Just enough of a `Hello` to check the version before anything else, so
/// an agent of another version gets a readable answer, not a parse error.
#[derive(Deserialize)]
struct HelloHeader {
    #[serde(rename = "type")]
    kind: String,
    protocol: u32,
}

async fn next_text(socket: &mut WebSocket) -> Option<String> {
    loop {
        match socket.recv().await? {
            Ok(Message::Text(text)) => return Some(text.as_str().to_owned()),
            Ok(Message::Ping(_) | Message::Pong(_)) => {}
            _ => return None,
        }
    }
}

/// Reads `Hello`, checks version and self-description, sends a fresh
/// challenge and verifies the signed answer. Says why on refusal.
async fn handshake(socket: &mut WebSocket) -> Option<Hello> {
    let text = next_text(socket).await?;
    if text.len() > MAX_HANDSHAKE_MESSAGE {
        reject(socket, "hello is too large".into()).await;
        return None;
    }
    let header: HelloHeader = serde_json::from_str(&text).ok()?;
    if header.kind != "hello" {
        return None;
    }
    if header.protocol != PROTOCOL_VERSION {
        let reason = format!(
            "protocol {} is not supported (expected {PROTOCOL_VERSION}); update this agent",
            header.protocol
        );
        reject(socket, reason).await;
        return None;
    }
    let Ok(AgentMessage::Hello {
        public_key,
        info,
        token,
        ..
    }) = serde_json::from_str::<AgentMessage>(&text)
    else {
        reject(socket, "malformed hello".into()).await;
        return None;
    };
    if let Err(e) = validate_node_info(&info) {
        reject(socket, e).await;
        return None;
    }
    let public_key = match PublicKey::parse(&public_key) {
        Ok(key) => key,
        Err(e) => {
            reject(socket, format!("public key: {e}")).await;
            return None;
        }
    };

    let nonce = random_hex(32).ok()?;
    send_master(
        socket,
        &MasterMessage::Challenge {
            nonce: nonce.clone(),
        },
    )
    .await
    .ok()?;
    let text = next_text(socket).await?;
    let signature = match serde_json::from_str::<AgentMessage>(&text) {
        Ok(AgentMessage::Proof { signature }) if text.len() <= MAX_HANDSHAKE_MESSAGE => signature,
        _ => {
            reject(socket, "expected a proof".into()).await;
            return None;
        }
    };
    if public_key
        .verify_hello(&nonce, &info.id, &signature)
        .is_err()
    {
        warn!("bad signature from {}", info.id);
        reject(socket, "signature does not match the key".into()).await;
        return None;
    }
    Some(Hello {
        info,
        public_key,
        token,
    })
}

fn handle_agent_message(cluster: &Cluster, node: &NodeId, msg: AgentMessage) {
    match msg {
        AgentMessage::Hello { .. } | AgentMessage::Proof { .. } => {
            warn!("agent {node} repeated a handshake message; ignoring")
        }
        AgentMessage::Metrics(m) => cluster.update_metrics(node, m),
        AgentMessage::TaskProgress {
            task_id,
            frames_done,
            ..
        } => cluster.task_progress(node, task_id, frames_done),
        AgentMessage::TaskDone { task_id } => cluster.task_done(node, task_id),
        AgentMessage::TaskFailed { task_id, error } => cluster.task_failed(node, task_id, error),
    }
}
