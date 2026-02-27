#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub max_streams: usize,
    pub max_viewers_per_stream: usize,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            max_streams: std::env::var("MAX_STREAMS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            max_viewers_per_stream: std::env::var("MAX_VIEWERS_PER_STREAM")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }
}
