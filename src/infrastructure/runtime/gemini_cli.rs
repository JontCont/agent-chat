use axum::{
    extract::Path,
    http::StatusCode,
    response::{sse::{Event, Sse}, IntoResponse},
    routing::{post, delete, get},
    Json, Router,
};
use futures_util::stream::StreamExt;
use std::convert::Infallible;
use serde::{Serialize, Deserialize};
use tracing::{info, error};
use crate::infrastructure::runtime::daemon_settings::DaemonSettings;
use crate::application::models::message::Attachment;

#[derive(Deserialize)]
pub struct DaemonMessageRequest {
    pub content: String,
    pub attachments: Option<Vec<Attachment>>,
    pub active_cli: Option<String>,
}

#[derive(Deserialize)]
pub struct ManualResponseRequest {
    pub content: String,
    pub attachments: Option<Vec<Attachment>>,
}

pub fn router() -> Router {
    Router::new()
        .route("/local/sessions/:id/messages", post(handle_messages))
        .route("/local/sessions/:id/cancel", post(handle_cancel))
        .route("/local/sessions/:id", delete(handle_delete))
        .route("/local/sessions/:id/history", get(get_session_history))
        .route("/local/sessions/:id/manual-response", post(post_manual_response))
        .route("/local/sessions/:id/human", post(handle_set_human))
        .route("/local/sessions/:id/ready", post(handle_set_ready))
        .route("/local/sessions/:id/sync-human", post(handle_sync_human))
        .route("/local/sessions/:id/sync-ready", post(handle_sync_ready))
        .route("/local/sessions", get(get_active_sessions))
        .route("/local/settings", post(post_settings).get(get_settings))
        .route("/", get(serve_settings_ui))
}

async fn handle_messages(
    Path(id): Path<String>,
    Json(payload): Json<DaemonMessageRequest>,
) -> impl axum::response::IntoResponse {
    info!("Daemon handling message run for session {}", id);
    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel(100);
    
    let settings = DaemonSettings::load();
    let active_cli = payload.active_cli.unwrap_or(settings.active_cli);
    let stream = crate::infrastructure::runtime::process_manager::spawn_run(id.clone(), payload.content, payload.attachments, active_cli);

    tokio::spawn(async move {
        let mut stream = stream;
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

async fn get_active_sessions() -> impl axum::response::IntoResponse {
    let registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
    let list: Vec<serde_json::Value> = registry
        .iter()
        .map(|(id, session)| {
            serde_json::json!({
                "id": id,
                "can_override": session.is_human || session.manual_tx.is_some(),
                "is_human": session.is_human
            })
        })
        .collect();
    (StatusCode::OK, Json(list)).into_response()
}

async fn get_session_history(Path(id): Path<String>) -> impl axum::response::IntoResponse {
    let registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
    if let Some(session) = registry.get(&id) {
        (StatusCode::OK, Json(session.history.clone())).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Session not found" }))).into_response()
    }
}

async fn handle_set_human(Path(id): Path<String>) -> StatusCode {
    info!("Setting session {} to human mode on Daemon", id);
    {
        let mut registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
        let session = registry.entry(id.clone()).or_insert_with(|| crate::infrastructure::runtime::process_manager::ActiveSession {
            manual_tx: None,
            history: Vec::new(),
            is_human: true,
        });
        session.is_human = true;
    }

    // Forward to Bridge POST /sessions/:id/human
    let bridge_base = bridge_base_url();
    let bridge_url = format!("{}/sessions/{}/human", bridge_base, id);
    let client = reqwest::Client::new();
    if let Err(e) = client.post(&bridge_url).send().await {
        tracing::warn!("Failed to notify Bridge of human status from Daemon: {:?}", e);
    }

    StatusCode::OK
}

async fn handle_set_ready(Path(id): Path<String>) -> StatusCode {
    info!("Setting session {} to ready (AI) mode on Daemon", id);
    {
        let mut registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
        if let Some(session) = registry.get_mut(&id) {
            session.is_human = false;
        }
    }

    // Forward to Bridge POST /sessions/:id/ready
    let bridge_base = bridge_base_url();
    let bridge_url = format!("{}/sessions/{}/ready", bridge_base, id);
    let client = reqwest::Client::new();
    if let Err(e) = client.post(&bridge_url).send().await {
        tracing::warn!("Failed to notify Bridge of ready status from Daemon: {:?}", e);
    }

    StatusCode::OK
}

async fn handle_sync_human(Path(id): Path<String>) -> StatusCode {
    info!("Syncing session {} to human mode from Bridge", id);
    let mut registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
    let session = registry.entry(id).or_insert_with(|| crate::infrastructure::runtime::process_manager::ActiveSession {
        manual_tx: None,
        history: Vec::new(),
        is_human: true,
    });
    session.is_human = true;
    StatusCode::OK
}

async fn handle_sync_ready(Path(id): Path<String>) -> StatusCode {
    info!("Syncing session {} to ready (AI) mode from Bridge", id);
    let mut registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
    if let Some(session) = registry.get_mut(&id) {
        session.is_human = false;
    }
    StatusCode::OK
}

async fn post_manual_response(
    Path(id): Path<String>,
    Json(payload): Json<ManualResponseRequest>,
) -> axum::response::Response {
    // Check if session is in human mode
    let is_human = {
        let registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
        registry.get(&id).map(|session| session.is_human).unwrap_or(false)
    };

    if !is_human {
        return (StatusCode::BAD_REQUEST, "Session is not in human intervention mode. Input is locked during AI mode.".to_string()).into_response();
    }

    // 1. Clone the sender out of the registry lock FIRST
    let manual_tx = {
        let registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
        registry.get(&id).and_then(|session| session.manual_tx.clone())
    };

    // 2. Terminate running CLI process if any
    if let Some(pid) = crate::infrastructure::runtime::process_manager::get_pid(&id) {
        let _ = crate::infrastructure::runtime::process_manager::terminate_process_tree(pid).await;
        crate::infrastructure::runtime::process_manager::delete_pid(&id);
    }

    let is_active = manual_tx.is_some();

    // 3. Send manual response lines
    if let Some(tx) = manual_tx {
        for line in payload.content.lines() {
            if tx.send(Ok(line.to_string())).await.is_err() {
                break;
            }
        }
    }

    // 4. Update session history and clear active sender
    {
        let mut registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
        if let Some(session) = registry.get_mut(&id) {
            session.history.push(crate::infrastructure::runtime::process_manager::DaemonMessage {
                role: "model".to_string(),
                content: payload.content.clone(),
                attachments: payload.attachments.clone(),
            });
            session.manual_tx = None;
        } else {
            return (StatusCode::NOT_FOUND, "Session not found".to_string()).into_response();
        }
    }

    // 5. Forward the operator response to the Bridge!
    // Construct the URL to Bridge API: http://127.0.0.1:8080/sessions/{id}/operator-response
    let bridge_base = bridge_base_url();
    let bridge_url = format!("{}/sessions/{}/operator-response", bridge_base, id);
    info!("Forwarding operator override response to Bridge: {}", bridge_url);

    let client = reqwest::Client::new();
    let res = client.post(&bridge_url)
        .json(&serde_json::json!({
            "content": payload.content,
            "attachments": payload.attachments
        }))
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => {
            info!("Successfully forwarded operator response to Bridge");
        }
        Ok(resp) => {
            tracing::warn!("Bridge returned error status on operator-response forwarding: {}", resp.status());
        }
        Err(e) => {
            tracing::warn!("Failed to forward operator response to Bridge (Bridge might be offline): {:?}", e);
        }
    }

    (StatusCode::OK, Json(serde_json::json!({ "streamed": is_active }))).into_response()
}

fn bridge_base_url() -> String {
    std::env::var("BRIDGE_URL").unwrap_or_else(|_| {
        let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
        format!("http://127.0.0.1:{}", port)
    })
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    id: String,
    session_id: String,
    task_type: String,
    payload: Option<String>,
    status: String,
}

#[derive(Serialize)]
struct ProgressRequest {
    line: String,
}

#[derive(Serialize)]
struct CompleteRequest {
    status: String,
    error: Option<String>,
}

async fn start_task_polling_loop() {
    let client = reqwest::Client::new();
    let bridge_base = bridge_base_url();
    let poll_url = format!("{}/sessions/tasks/poll", bridge_base);
    info!("Starting daemon task polling loop at {}", poll_url);

    loop {
        match client.get(&poll_url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<Option<Task>>().await {
                        Ok(Some(task)) => {
                            info!("Polled a task: {:?}", task);
                            let client_clone = client.clone();
                            let bridge_base_clone = bridge_base.clone();
                            tokio::spawn(async move {
                                if let Err(e) = execute_task(task, client_clone, bridge_base_clone).await {
                                    error!("Task execution failed: {:?}", e);
                                }
                            });
                        }
                        Ok(None) => {}
                        Err(e) => {
                            error!("Failed to parse poll response: {:?}", e);
                        }
                    }
                } else {
                    error!("API returned error status on poll: {}", resp.status());
                }
            }
            Err(e) => {
                error!("Connection to API server failed: {:?}", e);
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

async fn execute_task(task: Task, client: reqwest::Client, bridge_base: String) -> Result<(), String> {
    let complete_url = format!("{}/sessions/tasks/{}/complete", bridge_base, task.id);
    let progress_url = format!("{}/sessions/tasks/{}/progress", bridge_base, task.id);

    match task.task_type.as_str() {
        "run" => {
            let payload_str = task.payload.ok_or_else(|| "Missing payload for run task".to_string())?;
            let request: DaemonMessageRequest = serde_json::from_str(&payload_str)
                .map_err(|e| format!("Failed to parse run payload: {:?}", e))?;

            let settings = DaemonSettings::load();
            let active_cli = request.active_cli.unwrap_or(settings.active_cli);
            let mut stream = crate::infrastructure::runtime::process_manager::spawn_run(
                task.session_id.clone(),
                request.content,
                request.attachments,
                active_cli,
            );

            let mut accumulated = String::new();
            let mut failed = false;
            let mut err_msg = None;

            while let Some(res) = stream.next().await {
                match res {
                    Ok(line) => {
                        accumulated.push_str(&line);
                        accumulated.push('\n');

                        // Report event: delta
                        let _ = client.post(&progress_url)
                            .json(&ProgressRequest { line: "event: delta".to_string() })
                            .send()
                            .await;

                        // Report data line
                        let data_json = serde_json::json!({ "text": format!("{}\n", line) }).to_string();
                        let prog_resp = client.post(&progress_url)
                            .json(&ProgressRequest { line: format!("data: {}", data_json) })
                            .send()
                            .await;

                        if let Ok(r) = prog_resp {
                            #[derive(Deserialize)]
                            struct ProgressResponse {
                                #[serde(rename = "continue")]
                                cont: bool,
                            }
                            if let Ok(prog_data) = r.json::<ProgressResponse>().await {
                                if !prog_data.cont {
                                    info!("Task {} was cancelled. Terminating process.", task.id);
                                    if let Some(pid) = crate::infrastructure::runtime::process_manager::get_pid(&task.session_id) {
                                        let _ = crate::infrastructure::runtime::process_manager::terminate_process_tree(pid).await;
                                        crate::infrastructure::runtime::process_manager::delete_pid(&task.session_id);
                                    }
                                    failed = true;
                                    err_msg = Some("Task cancelled by user".to_string());
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        failed = true;
                        err_msg = Some(e.to_string());
                        break;
                    }
                }
            }

            if !failed {
                // Report event: done
                let _ = client.post(&progress_url)
                    .json(&ProgressRequest { line: "event: done".to_string() })
                    .send()
                    .await;

                let done_json = serde_json::json!({ "text": accumulated.trim_end() }).to_string();
                let _ = client.post(&progress_url)
                    .json(&ProgressRequest { line: format!("data: {}", done_json) })
                    .send()
                    .await;

                let _ = client.post(&complete_url)
                    .json(&CompleteRequest { status: "completed".to_string(), error: None })
                    .send()
                    .await;
            } else {
                let _ = client.post(&complete_url)
                    .json(&CompleteRequest { status: "failed".to_string(), error: err_msg })
                    .send()
                    .await;
            }
        }
        "cancel" => {
            if let Some(pid) = crate::infrastructure::runtime::process_manager::get_pid(&task.session_id) {
                let _ = crate::infrastructure::runtime::process_manager::terminate_process_tree(pid).await;
                crate::infrastructure::runtime::process_manager::delete_pid(&task.session_id);
            }
            let _ = client.post(&complete_url)
                .json(&CompleteRequest { status: "completed".to_string(), error: None })
                .send()
                .await;
        }
        "delete" => {
            if let Some(pid) = crate::infrastructure::runtime::process_manager::get_pid(&task.session_id) {
                let _ = crate::infrastructure::runtime::process_manager::terminate_process_tree(pid).await;
            }
            crate::infrastructure::runtime::process_manager::delete_pid(&task.session_id);
            let _ = client.post(&complete_url)
                .json(&CompleteRequest { status: "completed".to_string(), error: None })
                .send()
                .await;
        }
        "set_human" => {
            {
                let mut registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
                let session = registry.entry(task.session_id.clone()).or_insert_with(|| crate::infrastructure::runtime::process_manager::ActiveSession {
                    manual_tx: None,
                    history: Vec::new(),
                    is_human: true,
                });
                session.is_human = true;
            }
            let _ = client.post(&complete_url)
                .json(&CompleteRequest { status: "completed".to_string(), error: None })
                .send()
                .await;
        }
        "set_ready" => {
            {
                let mut registry = crate::infrastructure::runtime::process_manager::get_sessions_registry().lock().unwrap();
                if let Some(session) = registry.get_mut(&task.session_id) {
                    session.is_human = false;
                }
            }
            let _ = client.post(&complete_url)
                .json(&CompleteRequest { status: "completed".to_string(), error: None })
                .send()
                .await;
        }
        other => return Err(format!("Unknown task type: {}", other)),
    }

    Ok(())
}

pub async fn run_daemon(port: u16) {
    // Start background task polling loop
    tokio::spawn(async move {
        start_task_polling_loop().await;
    });

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
        let _guard = crate::infrastructure::runtime::daemon_settings::TEST_MUTEX.lock().unwrap();
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
        assert!(html_content.contains("<title>Daemon Settings & Human Intervention - Local Agent</title>"));
    }

    #[tokio::test]
    async fn test_manual_response_and_history() {
        let app = router();

        // 1. First trigger spawn_run by calling POST /local/sessions/session_test/messages
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/local/sessions/session_test/messages")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content": "hello_test", "active_cli": "human"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // 2. Query GET /local/sessions/session_test/history
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/local/sessions/session_test/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json.is_array());
        assert_eq!(json[0].get("role").unwrap().as_str().unwrap(), "user");
        assert_eq!(json[0].get("content").unwrap().as_str().unwrap(), "hello_test");

        // 3. Post POST /local/sessions/session_test/manual-response
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/local/sessions/session_test/manual-response")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content": "manual_override_response"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // 4. Query history again to ensure it contains model/override message
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/local/sessions/session_test/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 2);
        assert_eq!(json[1].get("role").unwrap().as_str().unwrap(), "model");
        assert_eq!(json[1].get("content").unwrap().as_str().unwrap(), "manual_override_response");
    }
}

