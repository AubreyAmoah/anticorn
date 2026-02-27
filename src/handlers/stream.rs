use axum::extract::{ws::Message, ws::WebSocket, State, WebSocketUpgrade};
use axum::response::Response;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::session::{generate_stream_id, StreamSession};
use crate::AppState;

#[derive(Deserialize)]
struct RegisterMsg {
    #[serde(rename = "type")]
    msg_type: String,
    stream_id: Option<String>,
}

#[derive(Serialize)]
struct RegisteredMsg<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    stream_id: &'a str,
}

pub async fn ws_stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_stream(socket, state))
}

async fn handle_stream(mut socket: WebSocket, state: Arc<AppState>) {
    // Wait for register message
    let stream_id = loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<RegisterMsg>(&text) {
                Ok(msg) if msg.msg_type == "register" => {
                    break msg.stream_id.unwrap_or_else(generate_stream_id);
                }
                _ => {}
            },
            Some(Ok(Message::Close(_))) | None => return,
            _ => {}
        }
    };

    // Check max streams limit
    {
        let sessions = state.sessions.read().await;
        if sessions.len() >= state.config.max_streams {
            let _ = socket
                .send(Message::Text(
                    r#"{"type":"error","message":"max streams reached"}"#.to_string(),
                ))
                .await;
            return;
        }
    }

    // Create and register session
    let session = StreamSession::new();
    let tx = session.tx.clone();
    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(stream_id.clone(), session);
    }
    tracing::info!("streamer registered: {}", stream_id);

    // Acknowledge
    let resp = serde_json::to_string(&RegisteredMsg {
        msg_type: "registered",
        stream_id: &stream_id,
    })
    .unwrap();
    if socket.send(Message::Text(resp)).await.is_err() {
        cleanup(&state, &stream_id).await;
        return;
    }

    // Relay binary frames
    loop {
        match socket.recv().await {
            Some(Ok(Message::Binary(data))) => {
                crate::relay::relay_frame(&tx, Bytes::from(data));
            }
            Some(Ok(Message::Ping(p))) => {
                let _ = socket.send(Message::Pong(p)).await;
            }
            Some(Ok(Message::Close(_))) | None => break,
            _ => {}
        }
    }

    tracing::info!("streamer disconnected: {}", stream_id);
    cleanup(&state, &stream_id).await;
}

async fn cleanup(state: &AppState, stream_id: &str) {
    let mut sessions = state.sessions.write().await;
    sessions.remove(stream_id);
}
