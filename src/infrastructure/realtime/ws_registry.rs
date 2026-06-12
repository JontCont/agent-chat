use crate::application::ports::ws_notifier::WsNotifier;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct WebSocketRegistry {
    senders: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>>,
}

impl WebSocketRegistry {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, session_id: String, tx: mpsc::UnboundedSender<String>) {
        info!("Registering WebSocket connection for session {}", session_id);
        let mut senders = self.senders.write().await;
        senders.insert(session_id, tx);
    }

    pub async fn unregister(&self, session_id: &str) {
        info!("Unregistering WebSocket connection for session {}", session_id);
        let mut senders = self.senders.write().await;
        senders.remove(session_id);
    }
}

impl WsNotifier for WebSocketRegistry {
    fn notify(&self, session_id: &str, event_json: String) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let senders = self.senders.clone();
        let session_id = session_id.to_string();
        Box::pin(async move {
            let senders_read = senders.read().await;
            if let Some(tx) = senders_read.get(&session_id) {
                let _ = tx.send(event_json);
            }
        })
    }
}
