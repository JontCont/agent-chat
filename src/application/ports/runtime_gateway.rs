use futures_util::Stream;
use std::io;

pub trait RuntimeGateway: Send + Sync {
    fn send_message(&self, session_id: &str, content: &str) -> impl Stream<Item = Result<String, io::Error>> + Send;
    fn cancel_run(&self, session_id: &str) -> impl std::future::Future<Output = Result<(), io::Error>> + Send;
    fn delete_session(&self, session_id: &str) -> impl std::future::Future<Output = Result<(), io::Error>> + Send;
}
