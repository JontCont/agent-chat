use crate::application::models::message::Message;

pub trait MessageRepository: Send + Sync {
    fn save(&self, message: &Message) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_by_session(&self, session_id: &str) -> impl std::future::Future<Output = Result<Vec<Message>, sqlx::Error>> + Send;
}
