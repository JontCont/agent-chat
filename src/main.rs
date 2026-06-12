pub mod api;
pub mod application;
pub mod infrastructure;

use crate::infrastructure::config::env::Config;
use crate::infrastructure::db::sqlite::init_db;
use crate::infrastructure::db::session_repository_impl::SqliteSessionRepository;
use crate::infrastructure::db::message_repository_impl::SqliteMessageRepository;
use crate::infrastructure::runtime::daemon_client::DaemonClient;
use crate::infrastructure::realtime::ws_registry::WebSocketRegistry;
use crate::application::services::session_service::SessionService;
use crate::application::services::runtime_service::RuntimeService;
use crate::application::services::cleanup_service::CleanupService;
use crate::api::AppState;

use std::sync::Arc;
use axum::{Router, routing::get};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize logging
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .try_init();

    let args: Vec<String> = std::env::args().collect();
    
    // Check if running in Daemon mode
    if args.contains(&"--daemon".to_string()) {
        info!("Starting Local Agent Daemon...");
        let port = std::env::var("DAEMON_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(7456);
        crate::infrastructure::runtime::gemini_cli::run_daemon(port).await;
        return;
    }

    info!("Starting Axum API Service...");
    let config = Config::from_env();

    // 1. Initialize SQLite Database Pool
    let pool = match init_db(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Database initialization failed: {:?}", e);
            std::process::exit(1);
        }
    };
    info!("Database connection established in WAL mode.");

    // 2. Instantiate Repositories, Clients, and Registries
    let session_repo = Arc::new(SqliteSessionRepository::new(pool.clone()));
    let message_repo = Arc::new(SqliteMessageRepository::new(pool.clone()));
    let daemon_client = Arc::new(DaemonClient::new(config.daemon_url));
    let ws_registry = Arc::new(WebSocketRegistry::new());

    // 3. Instantiate Application Services
    let session_service = Arc::new(SessionService::new(session_repo.clone()));
    let runtime_service = Arc::new(RuntimeService::new(
        session_repo.clone(),
        message_repo.clone(),
        daemon_client.clone(),
        ws_registry.clone(),
    ));
    let cleanup_service = Arc::new(CleanupService::new(session_repo.clone(), daemon_client.clone()));

    // 4. Start Background Cleanup Reaper (checks every 60 seconds)
    cleanup_service.clone().start_reaper(60);

    // 5. Build Shared AppState
    let state = Arc::new(AppState {
        session_service,
        runtime_service,
        ws_registry,
    });

    // 6. Build Axum Router
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/sessions", crate::api::routes::sessions::router())
        .nest("/ws", crate::api::routes::websocket::router())
        .fallback_service(ServeDir::new("src/frontend"))
        .with_state(state)
        .layer(CorsLayer::permissive());

    // 7. Start API Server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("Axum API Service listening on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
