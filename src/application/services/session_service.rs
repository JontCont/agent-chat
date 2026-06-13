use crate::application::models::session::{Session, SessionStatus};
use crate::application::ports::session_repository::SessionRepository;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct SessionService {
    session_repo: Arc<dyn SessionRepository>,
}

impl SessionService {
    pub fn new(session_repo: Arc<dyn SessionRepository>) -> Self {
        Self { session_repo }
    }

    pub async fn create_session(&self) -> Result<Session, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        // Expires in 30 minutes by default
        let expires_at = now + chrono::Duration::try_minutes(30).unwrap_or_default();

        let session = Session {
            id,
            status: SessionStatus::Starting,
            created_at: now,
            last_seen_at: now,
            expires_at,
            disconnected_at: None,
            runtime_id: None,
        };

        // Initialize state to Starting in SQLite
        self.session_repo.create(&session).await?;
        
        // Transition state to Ready in SQLite
        self.session_repo.update_status(&session.id, SessionStatus::Ready.as_str()).await?;
        
        let mut session = session;
        session.status = SessionStatus::Ready;
        Ok(session)
    }

    pub async fn get_session(&self, id: &str) -> Result<Option<Session>, sqlx::Error> {
        self.session_repo.get_by_id(id).await
    }

    pub async fn transfer_to_human(&self, id: &str) -> Result<(), sqlx::Error> {
        self.session_repo.update_status(id, SessionStatus::Human.as_str()).await
    }

    pub async fn restore_to_ready(&self, id: &str) -> Result<(), sqlx::Error> {
        self.session_repo.update_status(id, SessionStatus::Ready.as_str()).await
    }
}
