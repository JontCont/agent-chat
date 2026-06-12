use crate::application::models::session::{Session, SessionStatus};
use crate::application::ports::session_repository::SessionRepository;
use sqlx::{SqlitePool, Row};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct SqliteSessionRepository {
    pool: SqlitePool,
}

impl SqliteSessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl SessionRepository for SqliteSessionRepository {
    fn create(&self, session: &Session) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send>> {
        let pool = self.pool.clone();
        let session = session.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO sessions (id, status, created_at, last_seen_at, expires_at, disconnected_at, runtime_id) 
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&session.id)
            .bind(session.status.as_str())
            .bind(session.created_at.to_rfc3339())
            .bind(session.last_seen_at.to_rfc3339())
            .bind(session.expires_at.to_rfc3339())
            .bind(session.disconnected_at.map(|d| d.to_rfc3339()))
            .bind(&session.runtime_id)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn update_status(&self, id: &str, status: &str) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let status = status.to_string();
        Box::pin(async move {
            sqlx::query("UPDATE sessions SET status = ? WHERE id = ?")
                .bind(status)
                .bind(id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn update_expiry(&self, id: &str, last_seen_at: chrono::DateTime<chrono::Utc>, expires_at: chrono::DateTime<chrono::Utc>) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            sqlx::query("UPDATE sessions SET last_seen_at = ?, expires_at = ? WHERE id = ?")
                .bind(last_seen_at.to_rfc3339())
                .bind(expires_at.to_rfc3339())
                .bind(id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    fn get_by_id(&self, id: &str) -> Pin<Box<dyn Future<Output = Result<Option<Session>, sqlx::Error>> + Send>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT id, status, created_at, last_seen_at, expires_at, disconnected_at, runtime_id 
                 FROM sessions WHERE id = ?"
            )
            .bind(id)
            .fetch_optional(&pool)
            .await?;

            if let Some(r) = row {
                let status_str: String = r.get("status");
                let created_at_str: String = r.get("created_at");
                let last_seen_at_str: String = r.get("last_seen_at");
                let expires_at_str: String = r.get("expires_at");
                let disconnected_at_str: Option<String> = r.get("disconnected_at");
                let runtime_id: Option<String> = r.get("runtime_id");

                let status = SessionStatus::from_str(&status_str);
                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Utc);
                let last_seen_at = chrono::DateTime::parse_from_rfc3339(&last_seen_at_str)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Utc);
                let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Utc);
                let disconnected_at = disconnected_at_str.and_then(|d| {
                    chrono::DateTime::parse_from_rfc3339(&d)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                });

                Ok(Some(Session {
                    id: r.get("id"),
                    status,
                    created_at,
                    last_seen_at,
                    expires_at,
                    disconnected_at,
                    runtime_id,
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn get_expired(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Session>, sqlx::Error>> + Send>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let now = chrono::Utc::now().to_rfc3339();
            let rows = sqlx::query(
                "SELECT id, status, created_at, last_seen_at, expires_at, disconnected_at, runtime_id 
                 FROM sessions WHERE expires_at < ?"
            )
            .bind(now)
            .fetch_all(&pool)
            .await?;

            let mut sessions = Vec::new();
            for r in rows {
                let id: String = r.get("id");
                let status_str: String = r.get("status");
                let created_at_str: String = r.get("created_at");
                let last_seen_at_str: String = r.get("last_seen_at");
                let expires_at_str: String = r.get("expires_at");
                let disconnected_at_str: Option<String> = r.get("disconnected_at");
                let runtime_id: Option<String> = r.get("runtime_id");

                let status = SessionStatus::from_str(&status_str);
                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Utc);
                let last_seen_at = chrono::DateTime::parse_from_rfc3339(&last_seen_at_str)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Utc);
                let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Utc);
                let disconnected_at = disconnected_at_str.and_then(|d| {
                    chrono::DateTime::parse_from_rfc3339(&d)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                });

                sessions.push(Session {
                    id,
                    status,
                    created_at,
                    last_seen_at,
                    expires_at,
                    disconnected_at,
                    runtime_id,
                });
            }

            Ok(sessions)
        })
    }
}
