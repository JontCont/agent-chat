use crate::application::models::message::Message;
use std::future::Future;
use std::pin::Pin;

pub trait MessageRepository: Send + Sync {
    fn save(&self, message: &Message) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send>>;
    fn get_by_session(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<Vec<Message>, sqlx::Error>> + Send>>;
}
