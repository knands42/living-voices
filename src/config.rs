use std::path::PathBuf;
use std::io::Error as IoError;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub config_path: String,
}

impl Config {
    pub fn new() -> Result<Self, IoError> {
        let base_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let config_file_path = PathBuf::from(&base_dir)
            .join(".config")
            .join("living_voices")
            .join("config.toml");

        if !config_file_path.exists() {
            log::warn!(
                "Config not found at {}, creating default",
                config_file_path.display()
            );
            Self::create_default(&config_file_path)?;
        }

        Ok(Self::load_config(&config_file_path)
            .unwrap_or_else(|| Config::load_default_config(&base_dir)))
    }

    pub fn create_default(path: &std::path::Path) -> Result<(), IoError> {
        std::fs::create_dir_all(path.parent().unwrap_or(path))?;

        let content = include_str!("../config.toml");
        std::fs::write(path, content.as_bytes())?;

        log::info!("Default config created at {}", path.display());
        Ok(())
    }

    fn load_default_config(base_dir: &str) -> Config {
        Config {
            config_path: format!("{}/.config/living_voices/config.toml", base_dir),
        }
    }

    fn load_config(path: &PathBuf) -> Option<Config> {
        let text = std::fs::read_to_string(path).ok()?;
        let config: Config = toml::from_str(&text).ok()?;

        log::info!("Config loaded from {}", path.display());
        Some(config)
    }
}
