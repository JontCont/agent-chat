use crate::application::models::session::SessionStatus;
use crate::application::ports::session_repository::SessionRepository;
use crate::application::ports::runtime_gateway::RuntimeGateway;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, error};

pub struct CleanupService {
    session_repo: Arc<dyn SessionRepository>,
    runtime_gateway: Arc<dyn RuntimeGateway>,
}

impl CleanupService {
    pub fn new(
        session_repo: Arc<dyn SessionRepository>,
        runtime_gateway: Arc<dyn RuntimeGateway>,
    ) -> Self {
        Self {
            session_repo,
            runtime_gateway,
        }
    }

    pub async fn run_cleanup_cycle(&self) -> Result<(), String> {
        info!("Starting background session cleanup cycle...");
        let expired_sessions = match self.session_repo.get_expired().await {
            Ok(sessions) => sessions,
            Err(e) => return Err(format!("Failed to retrieve expired sessions from SQLite: {:?}", e)),
        };

        for session in expired_sessions {
            info!("Reaper cleaning up expired session: {}", session.id);
            
            // Call DELETE on the Daemon (Failure Mode is logged if Daemon is unreachable)
            if let Err(e) = self.runtime_gateway.delete_session(&session.id).await {
                error!(
                    "Daemon unreachable or error returned during cleanup for session {} (logged): {:?}",
                    session.id, e
                );
            }

            // Update state to Expired in SQLite
            if let Err(e) = self.session_repo.update_status(&session.id, SessionStatus::Expired.as_str()).await {
                error!("Failed to update SQLite session state to Expired for {}: {:?}", session.id, e);
            }
        }

        Ok(())
    }

    pub fn start_reaper(self: Arc<Self>, interval_secs: u64) {
        tokio::spawn(async move {
            info!("Background Cleanup Reaper loop started. Checking every {} seconds.", interval_secs);
            loop {
                sleep(Duration::from_secs(interval_secs)).await;
                if let Err(e) = self.run_cleanup_cycle().await {
                    error!("Reaper cleanup cycle encountered error: {}", e);
                }
            }
        });
    }
}
