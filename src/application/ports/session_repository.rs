use crate::application::models::session::Session;
use std::future::Future;
use std::pin::Pin;

pub trait SessionRepository: Send + Sync {
    fn create(&self, session: &Session) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send>>;
    fn update_status(&self, id: &str, status: &str) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send>>;
    fn update_expiry(&self, id: &str, last_seen_at: chrono::DateTime<chrono::Utc>, expires_at: chrono::DateTime<chrono::Utc>) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send>>;
    fn get_by_id(&self, id: &str) -> Pin<Box<dyn Future<Output = Result<Option<Session>, sqlx::Error>> + Send>>;
    fn get_expired(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Session>, sqlx::Error>> + Send>>;
}
