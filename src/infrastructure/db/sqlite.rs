use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;
use std::fs;
use std::path::Path;

pub async fn init_db(database_url: &str) -> Result<sqlx::SqlitePool, sqlx::Error> {
    // Extract folder path from database url to ensure parent directories exist
    // e.g. sqlite:///data/sqlite/agent.db -> /data/sqlite
    if let Some(path_str) = database_url.strip_prefix("sqlite://") {
        let path = Path::new(path_str);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
    }

    let connection_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    // Enable WAL mode programmatically as well to be absolutely sure
    sqlx::query("PRAGMA journal_mode = WAL;").execute(&pool).await?;

    // Create sessions table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            disconnected_at TEXT,
            runtime_id TEXT
        );"
    )
    .execute(&pool)
    .await?;

    // Create messages table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            is_final INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );"
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
