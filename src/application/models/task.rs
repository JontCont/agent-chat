use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub session_id: String,
    pub task_type: String, // "run", "cancel", "delete", "set_human", "set_ready"
    pub payload: Option<String>,
    pub status: String, // "pending", "running", "completed", "failed", "cancelled"
    pub created_at: String,
    pub updated_at: String,
}
