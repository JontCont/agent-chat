use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaemonSettings {
    pub active_cli: String,
}

impl Default for DaemonSettings {
    fn default() -> Self {
        Self {
            active_cli: "agy".to_string(),
        }
    }
}

impl DaemonSettings {
    pub fn load() -> Self {
        let path = Path::new("daemon_config.json");
        if !path.exists() {
            return Self::default();
        }
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        serde_json::from_str(&content).unwrap_or_else(|_| Self::default())
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Path::new("daemon_config.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_settings() {
        let settings = DaemonSettings::default();
        assert_eq!(settings.active_cli, "agy");
    }

    #[test]
    fn test_save_and_load_settings() {
        let config_file = Path::new("daemon_config.json");
        // Backup if file exists
        let backup = if config_file.exists() {
            Some(fs::read_to_string(config_file).unwrap())
        } else {
            None
        };

        let settings = DaemonSettings {
            active_cli: "openai".to_string(),
        };
        settings.save().unwrap();

        let loaded = DaemonSettings::load();
        assert_eq!(loaded.active_cli, "openai");

        // Clean up
        let _ = fs::remove_file(config_file);
        
        // Restore backup
        if let Some(content) = backup {
            fs::write(config_file, content).unwrap();
        }
    }
}
