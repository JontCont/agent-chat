use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    pub mime_type: String,
    pub data: String, // Base64 string
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String, // "user" or "model"
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub is_final: bool,
    pub attachments: Option<Vec<Attachment>>,
}
