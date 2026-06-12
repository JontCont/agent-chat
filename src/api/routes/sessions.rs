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
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_session))
        .route("/:id", get(get_session))
        .route("/:id/messages", post(post_message))
        .route("/:id/cancel", post(cancel_session))
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
    match state.runtime_service.run_message(&id, &payload.content).await {
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
