pub trait WsNotifier: Send + Sync {
    fn notify(&self, session_id: &str, event_json: String) -> impl std::future::Future<Output = ()> + Send;
}
