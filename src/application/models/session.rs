use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Starting,
    Ready,
    Busy,
    Disconnected,
    Expired,
    Closed,
    Faulted,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Starting => "starting",
            SessionStatus::Ready => "ready",
            SessionStatus::Busy => "busy",
            SessionStatus::Disconnected => "disconnected",
            SessionStatus::Expired => "expired",
            SessionStatus::Closed => "closed",
            SessionStatus::Faulted => "faulted",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "starting" => SessionStatus::Starting,
            "ready" => SessionStatus::Ready,
            "busy" => SessionStatus::Busy,
            "disconnected" => SessionStatus::Disconnected,
            "expired" => SessionStatus::Expired,
            "closed" => SessionStatus::Closed,
            "faulted" => SessionStatus::Faulted,
            _ => SessionStatus::Faulted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub disconnected_at: Option<DateTime<Utc>>,
    pub runtime_id: Option<String>,
}
