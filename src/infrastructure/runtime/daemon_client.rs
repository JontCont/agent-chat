use crate::application::ports::runtime_gateway::RuntimeGateway;
use crate::application::models::message::Attachment;
use futures_util::Stream;
use std::collections::HashMap;
use std::io;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::error;

#[derive(Clone)]
pub struct TaskStreamRegistry {
    senders: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Result<String, io::Error>>>>>,
}

impl TaskStreamRegistry {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, task_id: String, tx: mpsc::UnboundedSender<Result<String, io::Error>>) {
        let mut senders = self.senders.write().await;
        senders.insert(task_id, tx);
    }

    pub async fn unregister(&self, task_id: &str) {
        let mut senders = self.senders.write().await;
        senders.remove(task_id);
    }

    pub async fn send(&self, task_id: &str, line: Result<String, io::Error>) -> bool {
        let senders = self.senders.read().await;
        if let Some(tx) = senders.get(task_id) {
            tx.send(line).is_ok()
        } else {
            false
        }
    }
}

pub struct DaemonClient {
    pool: sqlx::SqlitePool,
    pub task_streams: Arc<TaskStreamRegistry>,
}

impl DaemonClient {
    pub fn new(pool: sqlx::SqlitePool, task_streams: Arc<TaskStreamRegistry>) -> Self {
        Self {
            pool,
            task_streams,
        }
    }
}

impl RuntimeGateway for DaemonClient {
    fn send_message(&self, session_id: &str, content: &str, attachments: Option<Vec<Attachment>>, active_cli: Option<String>) -> Pin<Box<dyn Stream<Item = Result<String, io::Error>> + Send>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let pool = self.pool.clone();
        let task_streams = self.task_streams.clone();
        
        let session_id = session_id.to_string();
        let content = content.to_string();
        let attachments = attachments.clone();
        let active_cli = active_cli.clone();

        tokio::spawn(async move {
            let task_id = uuid::Uuid::new_v4().to_string();
            
            // Register task sender in task_streams registry
            task_streams.register(task_id.clone(), tx.clone()).await;

            // Prepare payload JSON
            let payload = serde_json::json!({
                "content": content,
                "attachments": attachments,
                "active_cli": active_cli,
            }).to_string();

            let now = chrono::Utc::now().to_rfc3339();

            // Insert "run" task into SQLite
            let insert_res = sqlx::query(
                "INSERT INTO tasks (id, session_id, task_type, payload, status, created_at, updated_at)
                 VALUES (?, ?, 'run', ?, 'pending', ?, ?)"
            )
            .bind(&task_id)
            .bind(&session_id)
            .bind(&payload)
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;

            if let Err(e) = insert_res {
                error!("Failed to insert run task into SQLite: {:?}", e);
                let _ = tx.send(Err(io::Error::new(io::ErrorKind::Other, e.to_string())));
                task_streams.unregister(&task_id).await;
            }
        });

        // Convert unbounded receiver to stream
        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
    }

    fn cancel_run(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>> {
        let pool = self.pool.clone();
        let session_id = session_id.to_string();
        Box::pin(async move {
            let task_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            // Update any running/pending tasks for this session to 'cancelled'
            let update_res = sqlx::query(
                "UPDATE tasks SET status = 'cancelled', updated_at = ? WHERE session_id = ? AND status IN ('pending', 'running')"
            )
            .bind(&now)
            .bind(&session_id)
            .execute(&pool)
            .await;

            if let Err(e) = update_res {
                error!("Failed to update tasks status to cancelled: {:?}", e);
            }

            // Also insert a "cancel" task so that the Daemon receives it if it's polling
            let insert_res = sqlx::query(
                "INSERT INTO tasks (id, session_id, task_type, payload, status, created_at, updated_at)
                 VALUES (?, ?, 'cancel', NULL, 'pending', ?, ?)"
            )
            .bind(&task_id)
            .bind(&session_id)
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;

            match insert_res {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Failed to insert cancel task into SQLite: {:?}", e);
                    Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
                }
            }
        })
    }

    fn delete_session(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>> {
        let pool = self.pool.clone();
        let session_id = session_id.to_string();
        Box::pin(async move {
            let task_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            // Insert "delete" task
            let insert_res = sqlx::query(
                "INSERT INTO tasks (id, session_id, task_type, payload, status, created_at, updated_at)
                 VALUES (?, ?, 'delete', NULL, 'pending', ?, ?)"
            )
            .bind(&task_id)
            .bind(&session_id)
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;

            match insert_res {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Failed to insert delete task: {:?}", e);
                    Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
                }
            }
        })
    }

    fn set_human_mode(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>> {
        let pool = self.pool.clone();
        let session_id = session_id.to_string();
        Box::pin(async move {
            let task_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            // Insert "set_human" task
            let insert_res = sqlx::query(
                "INSERT INTO tasks (id, session_id, task_type, payload, status, created_at, updated_at)
                 VALUES (?, ?, 'set_human', NULL, 'pending', ?, ?)"
            )
            .bind(&task_id)
            .bind(&session_id)
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;

            match insert_res {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Failed to insert set_human task: {:?}", e);
                    Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
                }
            }
        })
    }

    fn set_ready_mode(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>> {
        let pool = self.pool.clone();
        let session_id = session_id.to_string();
        Box::pin(async move {
            let task_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            // Insert "set_ready" task
            let insert_res = sqlx::query(
                "INSERT INTO tasks (id, session_id, task_type, payload, status, created_at, updated_at)
                 VALUES (?, ?, 'set_ready', NULL, 'pending', ?, ?)"
            )
            .bind(&task_id)
            .bind(&session_id)
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;

            match insert_res {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Failed to insert set_ready task: {:?}", e);
                    Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
                }
            }
        })
    }
}
