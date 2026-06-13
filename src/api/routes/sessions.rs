use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
    routing::{post, get},
    Router,
};
use crate::api::AppState;
use crate::api::dto::session_dto::{CreateSessionResponse, SessionResponse};
use crate::api::dto::message_dto::PromptRequest;
use crate::application::ports::ws_notifier::WsNotifier;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_session))
        .route("/:id", get(get_session))
        .route("/:id/messages", post(post_message))
        .route("/:id/cancel", post(cancel_session))
        .route("/:id/human", post(transfer_to_human))
        .route("/:id/ready", post(restore_to_ready))
        .route("/:id/operator-response", post(post_operator_response))
}

async fn create_session(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match state.session_service.create_session().await {
        Ok(session) => {
            let resp = CreateSessionResponse {
                id: session.id,
                status: session.status.as_str().to_string(),
            };
            Ok((StatusCode::CREATED, Json(resp)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create session: {:?}", e),
        )),
    }
}

async fn get_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match state.session_service.get_session(&id).await {
        Ok(Some(session)) => {
            let resp = SessionResponse {
                id: session.id,
                status: session.status.as_str().to_string(),
                created_at: session.created_at.to_rfc3339(),
                expires_at: session.expires_at.to_rfc3339(),
            };
            Ok((StatusCode::OK, Json(resp)))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "Session not found".to_string())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to query session: {:?}", e),
        )),
    }
}

async fn post_message(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<PromptRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match state.runtime_service.run_message(&id, &payload.content, payload.attachments).await {
        Ok(()) => Ok(StatusCode::ACCEPTED),
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

async fn cancel_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match state.runtime_service.cancel_message(&id).await {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

async fn transfer_to_human(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match state.session_service.transfer_to_human(&id).await {
        Ok(()) => {
            if let Err(e) = state.runtime_service.notify_human_mode(&id).await {
                tracing::error!("Failed to notify Daemon about human mode: {:?}", e);
            }

            let event = serde_json::json!({
                "type": "session.status",
                "sessionId": id,
                "status": "human"
            }).to_string();
            state.ws_registry.notify(&id, event).await;
            Ok(StatusCode::OK)
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to transfer session: {:?}", e),
        )),
    }
}

async fn restore_to_ready(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match state.session_service.restore_to_ready(&id).await {
        Ok(()) => {
            if let Err(e) = state.runtime_service.notify_ready_mode(&id).await {
                tracing::error!("Failed to notify Daemon about ready mode: {:?}", e);
            }

            let event = serde_json::json!({
                "type": "session.status",
                "sessionId": id,
                "status": "ready"
            }).to_string();
            state.ws_registry.notify(&id, event).await;
            Ok(StatusCode::OK)
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to restore session to ready: {:?}", e),
        )),
    }
}

async fn post_operator_response(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<PromptRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let msg = crate::application::models::message::Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: id.clone(),
        role: "model".to_string(),
        content: payload.content.clone(),
        created_at: chrono::Utc::now(),
        is_final: true,
        attachments: payload.attachments.clone(),
    };

    match state.runtime_service.save_operator_message(&msg).await {
        Ok(()) => {
            let event = serde_json::json!({
                "type": "operator.message",
                "sessionId": id,
                "text": payload.content,
                "attachments": payload.attachments
            }).to_string();
            state.ws_registry.notify(&id, event).await;
            Ok(StatusCode::OK)
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save operator response: {:?}", e),
        )),
    }
}
