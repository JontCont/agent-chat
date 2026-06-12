use std::env;

pub struct Config {
    pub database_url: String,
    pub daemon_url: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:///data/sqlite/agent.db".to_string());
        
        let daemon_url = env::var("DAEMON_URL")
            .unwrap_or_else(|_| "http://host.docker.internal:7456".to_string());
        
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);

        Self {
            database_url,
            daemon_url,
            port,
        }
    }
}
