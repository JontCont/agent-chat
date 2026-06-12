use std::future::Future;
use std::pin::Pin;

pub trait WsNotifier: Send + Sync {
    fn notify(&self, session_id: &str, event_json: String) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}
