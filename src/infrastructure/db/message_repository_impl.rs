use crate::application::models::message::Message;
use crate::application::ports::message_repository::MessageRepository;
use sqlx::{SqlitePool, Row};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct SqliteMessageRepository {
    pool: SqlitePool,
}

impl SqliteMessageRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl MessageRepository for SqliteMessageRepository {
    fn save(&self, message: &Message) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send>> {
        let pool = self.pool.clone();
        let message = message.clone();
        Box::pin(async move {
            let attachments_json = message.attachments.as_ref().and_then(|a| serde_json::to_string(a).ok());
            sqlx::query(
                "INSERT INTO messages (id, session_id, role, content, created_at, is_final, attachments) 
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&message.id)
            .bind(&message.session_id)
            .bind(&message.role)
            .bind(&message.content)
            .bind(message.created_at.to_rfc3339())
            .bind(if message.is_final { 1 } else { 0 })
            .bind(attachments_json)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn get_by_session(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<Vec<Message>, sqlx::Error>> + Send>> {
        let pool = self.pool.clone();
        let session_id = session_id.to_string();
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, session_id, role, content, created_at, is_final, attachments 
                 FROM messages WHERE session_id = ? ORDER BY created_at ASC"
            )
            .bind(session_id)
            .fetch_all(&pool)
            .await?;

            let mut messages = Vec::new();
            for r in rows {
                let id: String = r.get("id");
                let session_id: String = r.get("session_id");
                let role: String = r.get("role");
                let content: String = r.get("content");
                let created_at_str: String = r.get("created_at");
                let is_final_int: i32 = r.get("is_final");
                let attachments_str: Option<String> = r.get("attachments");

                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                     .unwrap_or_default()
                     .with_timezone(&chrono::Utc);

                let attachments = attachments_str.and_then(|s| serde_json::from_str(&s).ok());

                messages.push(Message {
                    id,
                    session_id,
                    role,
                    content,
                    created_at,
                    is_final: is_final_int != 0,
                    attachments,
                });
            }

            Ok(messages)
        })
    }
}
