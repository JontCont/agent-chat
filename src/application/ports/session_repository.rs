use crate::application::models::session::Session;

pub trait SessionRepository: Send + Sync {
    fn create(&self, session: &Session) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn update_status(&self, id: &str, status: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn update_expiry(&self, id: &str, last_seen_at: chrono::DateTime<chrono::Utc>, expires_at: chrono::DateTime<chrono::Utc>) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_by_id(&self, id: &str) -> impl std::future::Future<Output = Result<Option<Session>, sqlx::Error>> + Send;
    fn get_expired(&self) -> impl std::future::Future<Output = Result<Vec<Session>, sqlx::Error>> + Send;
}
