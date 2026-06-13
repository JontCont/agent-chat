use futures_util::Stream;
use std::io;
use std::future::Future;
use std::pin::Pin;
use crate::application::models::message::Attachment;

pub trait RuntimeGateway: Send + Sync {
    fn send_message(&self, session_id: &str, content: &str, attachments: Option<Vec<Attachment>>, active_cli: Option<String>) -> Pin<Box<dyn Stream<Item = Result<String, io::Error>> + Send>>;
    fn cancel_run(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>>;
    fn delete_session(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>>;
    fn set_human_mode(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>>;
    fn set_ready_mode(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>>;
}
