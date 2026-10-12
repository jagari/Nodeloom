//! HTTP and WebSocket endpoints.
//!
//! * `GET  /api/state`  full [`ClusterSnapshot`]
//! * `POST /api/jobs`   submit a [`SubmitJob`]
//! * `GET  /ws/ui`      snapshot stream for UI clients
//! * `GET  /ws/agent`   agent connections
//!
//! UI endpoints need the pairing token as `Authorization: Bearer <token>` or,
//! for browsers' WebSocket API which cannot set headers, `?token=<token>`.
//! Traffic is plain HTTP/WS for now; do not expose the port beyond a trusted
//! network.

use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use nodeloom_core::auth::tokens_match;
use nodeloom_proto::{
    AgentMessage, ClusterSnapshot, JobView, MasterMessage, NodeId, PROTOCOL_VERSION, SubmitJob,
};
use serde::Deserialize;
use tokio::{net::TcpListener, sync::mpsc};
use tracing::{info, warn};

use crate::state::{Cluster, SubmitError};

const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_WS_MESSAGE: usize = 1 << 20;
const UI_PUSH_DELAY: Duration = Duration::from_millis(100);

pub fn router(cluster: Arc<Cluster>) -> Router {
    Router::new()
        .route("/api/state", get(get_state))
        .route("/api/jobs", post(post_job))
        .route("/ws/ui", get(ws_ui))
        .route("/ws/agent", get(ws_agent))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .with_state(cluster)
}

pub async fn serve(listener: TcpListener, cluster: Arc<Cluster>) -> std::io::Result<()> {
    axum::serve(listener, router(cluster)).await
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

fn authorized(cluster: &Cluster, headers: &HeaderMap, query: &TokenQuery) -> bool {
    let bearer = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match bearer.or(query.token.as_deref()) {
        Some(provided) => tokens_match(cluster.token(), provided),
        None => false,
    }
}

async fn get_state(
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Result<Json<ClusterSnapshot>, StatusCode> {
    if !authorized(&cluster, &headers, &query) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(cluster.snapshot()))
}

async fn post_job(
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
    Json(request): Json<SubmitJob>,
) -> Result<Json<JobView>, Response> {
    if !authorized(&cluster, &headers, &query) {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    }
    cluster.submit_job(request.payload).map(Json).map_err(|e| {
        let status = match e {
            SubmitError::Invalid(_) => StatusCode::BAD_REQUEST,
            SubmitError::NoCapableNodes(_) => StatusCode::CONFLICT,
        };
        (status, e.to_string()).into_response()
    })
}

async fn ws_ui(
    ws: WebSocketUpgrade,
    State(cluster): State<Arc<Cluster>>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Response {
    if !authorized(&cluster, &headers, &query) {
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

async fn ws_agent(ws: WebSocketUpgrade, State(cluster): State<Arc<Cluster>>) -> Response {
    ws.max_message_size(MAX_WS_MESSAGE)
        .on_upgrade(move |socket| agent_session(socket, cluster))
}

async fn send_master(socket: &mut WebSocket, msg: &MasterMessage) -> Result<(), axum::Error> {
    let json = serde_json::to_string(msg).expect("MasterMessage serializes");
    socket.send(Message::Text(json.into())).await
}

async fn agent_session(mut socket: WebSocket, cluster: Arc<Cluster>) {
    let Some(info) = handshake(&mut socket, &cluster).await else {
        return;
    };
    let node_id: NodeId = info.id.clone();
    info!(
        "agent {} ({}, {}) connected",
        node_id, info.hostname, info.arch
    );

    let (tx, mut outbox) = mpsc::unbounded_channel();
    let generation = cluster.register_node(info, tx);

    loop {
        tokio::select! {
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

    cluster.unregister_node(&node_id, generation);
    info!("agent {node_id} disconnected");
}

/// Reads the `Hello`, checks protocol version and token, and answers
/// `Welcome` or `Rejected`. Returns the node description on success.
async fn handshake(socket: &mut WebSocket, cluster: &Cluster) -> Option<nodeloom_proto::NodeInfo> {
    let first = tokio::time::timeout(HELLO_TIMEOUT, socket.recv())
        .await
        .ok()??;
    let Ok(Message::Text(text)) = first else {
        return None;
    };
    let Ok(AgentMessage::Hello {
        protocol,
        token,
        info,
    }) = serde_json::from_str::<AgentMessage>(text.as_str())
    else {
        return None;
    };

    let rejection = if protocol != PROTOCOL_VERSION {
        Some(format!(
            "protocol {protocol} is not supported (expected {PROTOCOL_VERSION})"
        ))
    } else if !tokens_match(cluster.token(), &token) {
        Some("wrong pairing token".to_owned())
    } else {
        None
    };
    if let Some(reason) = rejection {
        warn!("rejected agent {}: {reason}", info.id);
        let _ = send_master(socket, &MasterMessage::Rejected { reason }).await;
        return None;
    }
    send_master(socket, &MasterMessage::Welcome).await.ok()?;
    Some(info)
}

fn handle_agent_message(cluster: &Cluster, node: &NodeId, msg: AgentMessage) {
    match msg {
        AgentMessage::Hello { .. } => warn!("agent {node} sent a second Hello; ignoring"),
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
