//! Checks that an agent notices a master that stopped answering (say, a
//! laptop that went to sleep or lost its Wi-Fi) and connects again.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use nodeloom_agent::{AgentConfig, AgentStatus, Identity, run_forever};
use nodeloom_core::RunnerSet;
use nodeloom_proto::{AgentMessage, MasterMessage};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

/// Completes the handshake (`Hello` -> `Challenge` -> `Proof` -> `Welcome`)
/// and returns the open WebSocket so the caller can decide whether to keep
/// reading from it or let it go silent.
async fn accept_and_welcome(listener: &TcpListener) -> WebSocketStream<TcpStream> {
    let (tcp, _) = listener.accept().await.unwrap();
    let mut ws = accept_async(tcp).await.unwrap();

    let hello = next_agent_message(&mut ws).await;
    assert!(matches!(hello, AgentMessage::Hello { .. }), "{hello:?}");

    send_master(
        &mut ws,
        &MasterMessage::Challenge {
            nonce: "0123456789abcdef".into(),
        },
    )
    .await;

    let proof = next_agent_message(&mut ws).await;
    assert!(matches!(proof, AgentMessage::Proof { .. }), "{proof:?}");

    send_master(&mut ws, &MasterMessage::Welcome).await;
    ws
}

async fn next_agent_message(ws: &mut WebSocketStream<TcpStream>) -> AgentMessage {
    while let Some(Ok(frame)) = ws.next().await {
        if let Message::Text(text) = frame {
            return serde_json::from_str(text.as_str()).unwrap();
        }
    }
    panic!("agent closed the connection during handshake");
}

async fn send_master(ws: &mut WebSocketStream<TcpStream>, msg: &MasterMessage) {
    ws.send(Message::text(serde_json::to_string(msg).unwrap()))
        .await
        .unwrap();
}

#[tokio::test]
async fn reconnects_when_the_master_stops_answering() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let master_url = format!("ws://{}/ws/agent", listener.local_addr().unwrap());

    let (status_tx, mut status_rx) = watch::channel(AgentStatus::Searching);
    let config = AgentConfig {
        master_url: master_url.clone(),
        identity: Identity::generate().unwrap(),
        token: None,
        runners: RunnerSet::new(),
        metrics_interval: Duration::from_millis(50),
        status: Some(status_tx),
    };
    let agent = tokio::spawn(async move {
        run_forever(config).await;
    });

    // 1. Complete the handshake and send Welcome, then never read from the
    //    socket again (so pings from the agent are never answered with Pong).
    let _stalled_socket = accept_and_welcome(&listener).await;

    tokio::time::timeout(
        Duration::from_secs(5),
        status_rx.wait_for(|s| matches!(s, AgentStatus::Joined { .. })),
    )
    .await
    .expect("timed out waiting for Joined")
    .unwrap();

    // 2. Once the idle timeout expires, the agent reports Connecting with
    //    `last_error: Some("the master stopped answering")`.
    let status = tokio::time::timeout(
        Duration::from_secs(10),
        status_rx.wait_for(|s| {
            matches!(
                s,
                AgentStatus::Connecting {
                    last_error: Some(_),
                    ..
                }
            )
        }),
    )
    .await
    .expect("timed out waiting for the agent to notice the silent master")
    .unwrap()
    .clone();

    match status {
        AgentStatus::Connecting { master, last_error } => {
            assert_eq!(master, master_url);
            let err = last_error.expect("expected last_error");
            assert!(
                err.contains("the master stopped answering"),
                "unexpected error: {err}"
            );
        }
        other => panic!("expected Connecting, got {other:?}"),
    }

    // 3. The agent then connects a second time and joins again.
    let mut second_socket =
        tokio::time::timeout(Duration::from_secs(10), accept_and_welcome(&listener))
            .await
            .expect("timed out waiting for the agent to reconnect");

    let keep_reading =
        tokio::spawn(async move { while let Some(Ok(_)) = second_socket.next().await {} });

    tokio::time::timeout(
        Duration::from_secs(5),
        status_rx.wait_for(|s| matches!(s, AgentStatus::Joined { .. })),
    )
    .await
    .expect("timed out waiting for Joined on the second connection")
    .unwrap();

    agent.abort();
    keep_reading.abort();
}
