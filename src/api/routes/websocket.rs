use axum::{
    extract::{Path, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use crate::api::AppState;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use futures_util::{StreamExt, SinkExt};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/:id", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Check if session exists
    match state.session_service.get_session(&id).await {
        Ok(Some(_session)) => {
            ws.on_upgrade(move |socket| handle_socket(socket, id, state))
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, "Session not found").into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve session: {:?}", e),
            )
                .into_response()
        }
    }
}

async fn handle_socket(socket: WebSocket, session_id: String, state: Arc<AppState>) {
    info!("WebSocket upgrade request accepted for session: {}", session_id);
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    // Register the connection in the registry
    state.ws_registry.register(session_id.clone(), tx).await;
    
    // Spawn task to forward events from the registry queue to the client
    let session_id_clone = session_id.clone();
    let ws_registry_clone = state.ws_registry.clone();
    
    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Some(msg_str) = rx.recv().await {
            if sender.send(Message::Text(msg_str)).await.is_err() {
                break;
            }
        }
    });

    // Receive loop: keep socket open by receiving (and discarding client inputs)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(_)) = receiver.next().await {
            // Discard client incoming frames (as WS is outbound-only streaming in this phase)
        }
    });

    tokio::select! {
        _ = (&mut send_task) => {}
        _ = (&mut recv_task) => {}
    }

    // Unregister upon disconnection
    ws_registry_clone.unregister(&session_id_clone).await;
}
