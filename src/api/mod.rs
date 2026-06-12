pub mod routes;
pub mod dto;
pub mod errors;

use crate::application::services::session_service::SessionService;
use crate::application::services::runtime_service::RuntimeService;
use crate::infrastructure::realtime::ws_registry::WebSocketRegistry;
use std::sync::Arc;

pub struct AppState {
    pub session_service: Arc<SessionService>,
    pub runtime_service: Arc<RuntimeService>,
    pub ws_registry: Arc<WebSocketRegistry>,
}
