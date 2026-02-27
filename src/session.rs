use bytes::Bytes;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio::sync::{broadcast, RwLock};

pub type StreamId = String;

const BROADCAST_CAPACITY: usize = 32;

pub struct StreamSession {
    pub tx: broadcast::Sender<Bytes>,
    pub viewer_count: Arc<AtomicUsize>,
}

impl StreamSession {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            tx,
            viewer_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

pub type SessionStore = Arc<RwLock<HashMap<StreamId, StreamSession>>>;

pub fn generate_stream_id() -> StreamId {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect()
}
