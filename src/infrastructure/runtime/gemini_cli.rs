use axum::{
    extract::Path,
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{post, delete, get},
    Json, Router,
};
use futures_util::stream::StreamExt;
use std::convert::Infallible;
use serde::{Serialize, Deserialize};
use tracing::{info, error};
use crate::infrastructure::runtime::daemon_settings::DaemonSettings;

#[derive(Deserialize)]
pub struct DaemonMessageRequest {
    pub content: String,
}

pub fn router() -> Router {
    Router::new()
        .route("/local/sessions/:id/messages", post(handle_messages))
        .route("/local/sessions/:id/cancel", post(handle_cancel))
        .route("/local/sessions/:id", delete(handle_delete))
        .route("/local/settings", post(post_settings).get(get_settings))
        .route("/", get(serve_settings_ui))
}

async fn handle_messages(
    Path(id): Path<String>,
    Json(payload): Json<DaemonMessageRequest>,
) -> impl axum::response::IntoResponse {
    info!("Daemon handling message run for session {}", id);
    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel(100);
    
    tokio::spawn(async move {
        let settings = DaemonSettings::load();
        let mut stream = crate::infrastructure::runtime::process_manager::spawn_run(id, payload.content, settings.active_cli);
        let mut accumulated = String::new();
        
        while let Some(res) = stream.next().await {
            match res {
                Ok(line) => {
                    accumulated.push_str(&line);
                    accumulated.push('\n');
                    let event_data = serde_json::json!({ "text": format!("{}\n", line) }).to_string();
                    if sse_tx.send(Ok::<_, Infallible>(Event::default().event("delta").data(event_data))).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let event_data = serde_json::json!({ "error": e.to_string() }).to_string();
                    let _ = sse_tx.send(Ok(Event::default().event("error").data(event_data))).await;
                    return;
                }
            }
        }
        
        // Yield done event with the full accumulated response text
        let done_data = serde_json::json!({ "text": accumulated.trim_end() }).to_string();
        let _ = sse_tx.send(Ok(Event::default().event("done").data(done_data))).await;
    });

    let sse_stream = tokio_stream::wrappers::ReceiverStream::new(sse_rx);
    Sse::new(sse_stream).keep_alive(axum::response::sse::KeepAlive::default())
}

async fn handle_cancel(Path(id): Path<String>) -> Result<axum::response::Response, (StatusCode, String)> {
    info!("Daemon handling cancel for session {}", id);
    if let Some(pid) = crate::infrastructure::runtime::process_manager::get_pid(&id) {
        if let Err(e) = crate::infrastructure::runtime::process_manager::terminate_process_tree(pid).await {
            error!("Failed to terminate process tree for session {}: {:?}", id, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to kill process: {:?}", e)));
        }
        crate::infrastructure::runtime::process_manager::delete_pid(&id);
        Ok(axum::response::IntoResponse::into_response(StatusCode::OK))
    } else {
        Ok(axum::response::IntoResponse::into_response((StatusCode::NOT_FOUND, "No active process found for session".to_string())))
    }
}

async fn handle_delete(Path(id): Path<String>) -> Result<impl axum::response::IntoResponse, (StatusCode, String)> {
    info!("Daemon handling DELETE for session {}", id);
    if let Some(pid) = crate::infrastructure::runtime::process_manager::get_pid(&id) {
        let _ = crate::infrastructure::runtime::process_manager::terminate_process_tree(pid).await;
    }
    crate::infrastructure::runtime::process_manager::delete_pid(&id);
    Ok(StatusCode::OK)
}

#[derive(Serialize, Deserialize)]
pub struct LocalSettingsPayload {
    pub active_cli: String,
}

async fn get_settings() -> impl axum::response::IntoResponse {
    let settings = DaemonSettings::load();
    let payload = LocalSettingsPayload {
        active_cli: settings.active_cli,
    };
    (StatusCode::OK, Json(payload))
}

async fn post_settings(
    Json(payload): Json<LocalSettingsPayload>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, String)> {
    let valid_clis = ["agy", "openai", "copilot", "claude"];
    if !valid_clis.contains(&payload.active_cli.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid active_cli name".to_string()));
    }
    let settings = DaemonSettings {
        active_cli: payload.active_cli,
    };
    if let Err(e) = settings.save() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save settings: {:?}", e)));
    }
    Ok(StatusCode::OK)
}

async fn serve_settings_ui() -> impl axum::response::IntoResponse {
    let html = include_str!("settings_ui.html");
    axum::response::Html(html)
}

pub async fn run_daemon(port: u16) {
    let app = router();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await.unwrap();
    info!("Local Agent Daemon listening on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use serde_json::Value;

    #[tokio::test]
    async fn test_settings_routes() {
        let app = router();

        // 1. Test GET /local/settings
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/local/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("active_cli").is_some());

        // 2. Test POST /local/settings with invalid CLI
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/local/settings")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"active_cli": "invalid"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // 3. Test POST /local/settings with valid CLI
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/local/settings")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"active_cli": "openai"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify it was updated
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/local/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.get("active_cli").unwrap().as_str().unwrap(), "openai");

        // Cleanup
        let _ = std::fs::remove_file("daemon_config.json");
    }

    #[tokio::test]
    async fn test_settings_ui_route() {
        let app = router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let html_content = String::from_utf8(body.to_vec()).unwrap();
        assert!(html_content.contains("<title>Daemon Settings - Local Agent</title>"));
    }
}

