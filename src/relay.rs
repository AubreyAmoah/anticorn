use bytes::Bytes;
use tokio::sync::broadcast;

pub fn relay_frame(tx: &broadcast::Sender<Bytes>, data: Bytes) {
    if tx.receiver_count() == 0 {
        return;
    }
    // Ignore errors: lagged or no receivers
    let _ = tx.send(data);
}
