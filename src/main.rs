use axum::{routing::get, Router};
use tower_http::services::ServeFile;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod handlers;
mod relay;
mod session;

use config::Config;
use session::SessionStore;

pub struct AppState {
    pub sessions: SessionStore,
    pub config: Config,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            ),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let addr = format!("{}:{}", config.host, config.port);

    let state = Arc::new(AppState {
        sessions: Arc::new(RwLock::new(HashMap::new())),
        config,
    });

    let app = Router::new()
        .route("/ws/stream", get(handlers::stream::ws_stream_handler))
        .route("/ws/view", get(handlers::view::ws_view_handler))
        .route("/health", get(|| async { "ok" }))
        .route_service("/blocklist.txt", ServeFile::new("blocklist.txt"))
        .with_state(state);

    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
