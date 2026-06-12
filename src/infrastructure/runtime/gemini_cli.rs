use axum::{
    extract::Path,
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{post, delete},
    Json, Router,
};
use futures_util::stream::StreamExt;
use std::convert::Infallible;
use serde::Deserialize;
use tracing::{info, error};

#[derive(Deserialize)]
pub struct DaemonMessageRequest {
    pub content: String,
}

pub fn router() -> Router {
    Router::new()
        .route("/local/sessions/:id/messages", post(handle_messages))
        .route("/local/sessions/:id/cancel", post(handle_cancel))
        .route("/local/sessions/:id", delete(handle_delete))
}

async fn handle_messages(
    Path(id): Path<String>,
    Json(payload): Json<DaemonMessageRequest>,
) -> impl axum::response::IntoResponse {
    info!("Daemon handling message run for session {}", id);
    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel(100);
    
    tokio::spawn(async move {
        let mut stream = crate::infrastructure::runtime::process_manager::spawn_run(id, payload.content);
        let mut accumulated = String::new();
        
        while let Some(res) = stream.next().await {
            match res {
                Ok(line) => {
                    accumulated.push_str(&line);
                    accumulated.push('\n');
                    let event_data = serde_json::json!({ "text": line }).to_string();
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

async fn handle_cancel(Path(id): Path<String>) -> Result<impl axum::response::IntoResponse, (StatusCode, String)> {
    info!("Daemon handling cancel for session {}", id);
    if let Some(pid) = crate::infrastructure::runtime::process_manager::get_pid(&id) {
        if let Err(e) = crate::infrastructure::runtime::process_manager::terminate_process_tree(pid).await {
            error!("Failed to terminate process tree for session {}: {:?}", id, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to kill process: {:?}", e)));
        }
        crate::infrastructure::runtime::process_manager::delete_pid(&id);
        Ok(StatusCode::OK)
    } else {
        Ok((StatusCode::NOT_FOUND, "No active process found for session".to_string()))
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

pub async fn run_daemon(port: u16) {
    let app = router();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await.unwrap();
    info!("Local Agent Daemon listening on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await.unwrap();
}
