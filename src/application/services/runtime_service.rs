use crate::application::models::session::SessionStatus;
use crate::application::models::message::Message;
use crate::application::ports::session_repository::SessionRepository;
use crate::application::ports::message_repository::MessageRepository;
use crate::application::ports::runtime_gateway::RuntimeGateway;
use crate::application::ports::ws_notifier::WsNotifier;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use futures_util::StreamExt;
use tracing::{info, error};

pub struct RuntimeService {
    session_repo: Arc<dyn SessionRepository>,
    message_repo: Arc<dyn MessageRepository>,
    runtime_gateway: Arc<dyn RuntimeGateway>,
    ws_notifier: Arc<dyn WsNotifier>,
}

impl RuntimeService {
    pub fn new(
        session_repo: Arc<dyn SessionRepository>,
        message_repo: Arc<dyn MessageRepository>,
        runtime_gateway: Arc<dyn RuntimeGateway>,
        ws_notifier: Arc<dyn WsNotifier>,
    ) -> Self {
        Self {
            session_repo,
            message_repo,
            runtime_gateway,
            ws_notifier,
        }
    }

    pub async fn run_message(&self, session_id: &str, content: &str) -> Result<(), String> {
        let session = match self.session_repo.get_by_id(session_id).await {
            Ok(Some(s)) => s,
            Ok(None) => return Err("Session not found".to_string()),
            Err(e) => return Err(format!("Database error: {:?}", e)),
        };

        if session.status != SessionStatus::Ready && session.status != SessionStatus::Disconnected {
            return Err("Session is busy or in invalid state".to_string());
        }

        // 1. Update session status to Busy in SQLite
        if let Err(e) = self.session_repo.update_status(session_id, SessionStatus::Busy.as_str()).await {
            return Err(format!("Failed to update session status: {:?}", e));
        }

        // 2. Save user prompt message to SQLite
        let user_msg = Message {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            created_at: Utc::now(),
            is_final: true,
        };
        let _ = self.message_repo.save(&user_msg).await;

        // 3. Connect to Daemon and receive streaming events
        let gateway = self.runtime_gateway.clone();
        let ws_notifier = self.ws_notifier.clone();
        let session_repo = self.session_repo.clone();
        let message_repo = self.message_repo.clone();
        let session_id_str = session_id.to_string();

        tokio::spawn(async move {
            info!("Starting prompt execution loop for session: {}", session_id_str);
            let mut stream = gateway.send_message(&session_id_str, &user_msg.content);
            let mut full_response = String::new();
            let mut current_event = String::new();

            while let Some(res) = stream.next().await {
                match res {
                    Ok(line) => {
                        // Forward raw SSE line directly to WS client
                        ws_notifier.notify(&session_id_str, line.clone()).await;

                        // Parse output to build final full_response message
                        if line.starts_with("event: ") {
                            current_event = line["event: ".len()..].trim().to_string();
                        } else if line.starts_with("data: ") {
                            let data_part = &line["data: ".len()..];
                            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(data_part) {
                                if let Some(text) = json_val.get("text").and_then(|t| t.as_str()) {
                                    if current_event == "done" {
                                        full_response = text.to_string();
                                    } else if current_event == "delta" {
                                        full_response.push_str(text);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Daemon connection error during streaming: {:?}", e);
                        // Push error event to WebSocket client
                        let err_json = serde_json::json!({
                            "type": "session.error",
                            "sessionId": session_id_str,
                            "error": format!("Daemon communication failure: {}", e)
                        }).to_string();
                        ws_notifier.notify(&session_id_str, err_json).await;

                        let _ = session_repo.update_status(&session_id_str, SessionStatus::Faulted.as_str()).await;
                        return;
                    }
                }
            }

            // Save final generated response message to SQLite
            let ai_msg = Message {
                id: Uuid::new_v4().to_string(),
                session_id: session_id_str.clone(),
                role: "model".to_string(),
                content: full_response,
                created_at: Utc::now(),
                is_final: true,
            };
            
            if let Err(e) = message_repo.save(&ai_msg).await {
                error!("Failed to persist AI message response to SQLite: {:?}", e);
            }

            // Revert session state back to Ready and extend expiry
            let now = Utc::now();
            let new_expires_at = now + chrono::Duration::try_minutes(30).unwrap_or_default();
            let _ = session_repo.update_expiry(&session_id_str, now, new_expires_at).await;
            let _ = session_repo.update_status(&session_id_str, SessionStatus::Ready.as_str()).await;
            info!("Session state reset to Ready for: {}", session_id_str);
        });

        Ok(())
    }

    pub async fn cancel_message(&self, session_id: &str) -> Result<(), String> {
        info!("Canceling execution for session: {}", session_id);
        
        // Notify Daemon to terminate the child process tree
        if let Err(e) = self.runtime_gateway.cancel_run(session_id).await {
            return Err(format!("Failed to cancel run on Daemon: {:?}", e));
        }

        // Notify client WebSocket
        let cancel_event = serde_json::json!({
            "type": "session.ready",
            "sessionId": session_id,
            "status": "ready"
        }).to_string();
        self.ws_notifier.notify(session_id, cancel_event).await;

        // Reset session state to Ready
        if let Err(e) = self.session_repo.update_status(session_id, SessionStatus::Ready.as_str()).await {
            return Err(format!("Failed to update SQLite session state to Ready: {:?}", e));
        }

        Ok(())
    }
}
