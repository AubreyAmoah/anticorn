use axum::extract::{ws::Message, ws::WebSocket, State, WebSocketUpgrade};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use std::sync::{atomic::Ordering, Arc};
use tokio::sync::broadcast;

use crate::AppState;

#[derive(Deserialize)]
struct SubscribeMsg {
    #[serde(rename = "type")]
    msg_type: String,
    stream_id: String,
}

#[derive(Serialize)]
struct SubscribedMsg<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    stream_id: &'a str,
}

pub async fn ws_view_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_view(socket, state))
}

async fn handle_view(mut socket: WebSocket, state: Arc<AppState>) {
    // Wait for subscribe message
    let stream_id = loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<SubscribeMsg>(&text) {
                Ok(msg) if msg.msg_type == "subscribe" => break msg.stream_id,
                _ => {}
            },
            Some(Ok(Message::Close(_))) | None => return,
            _ => {}
        }
    };

    // Look up stream and check viewer cap
    let (tx, viewer_count) = {
        let sessions = state.sessions.read().await;
        match sessions.get(&stream_id) {
            Some(s) => (s.tx.clone(), Arc::clone(&s.viewer_count)),
            None => {
                let _ = socket
                    .send(Message::Text(
                        r#"{"type":"error","message":"stream not found"}"#.to_string(),
                    ))
                    .await;
                return;
            }
        }
    };

    let prev = viewer_count.fetch_add(1, Ordering::Relaxed);
    if prev >= state.config.max_viewers_per_stream {
        viewer_count.fetch_sub(1, Ordering::Relaxed);
        let _ = socket
            .send(Message::Text(
                r#"{"type":"error","message":"max viewers reached"}"#.to_string(),
            ))
            .await;
        return;
    }

    let mut rx = tx.subscribe();
    tracing::info!("viewer subscribed to: {}", stream_id);

    // Acknowledge
    let resp = serde_json::to_string(&SubscribedMsg {
        msg_type: "subscribed",
        stream_id: &stream_id,
    })
    .unwrap();
    if socket.send(Message::Text(resp)).await.is_err() {
        viewer_count.fetch_sub(1, Ordering::Relaxed);
        return;
    }

    // Forward frames until stream ends or viewer disconnects
    loop {
        tokio::select! {
            frame = rx.recv() => match frame {
                Ok(data) => {
                    if socket.send(Message::Binary(data.to_vec())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    let _ = socket.send(Message::Text(
                        r#"{"type":"stream_ended"}"#.to_string(),
                    )).await;
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("viewer lagged {} frames on stream {}", n, stream_id);
                }
            },
            msg = socket.recv() => match msg {
                Some(Ok(Message::Close(_))) | None => break,
                _ => {}
            },
        }
    }

    tracing::info!("viewer disconnected from: {}", stream_id);
    viewer_count.fetch_sub(1, Ordering::Relaxed);
}
