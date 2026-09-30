use std::io::Error as IoError;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub config_path: String,
    pub db_path: String,
}

impl Config {
    pub fn new() -> Result<Self, IoError> {
        let home_dir = Config::get_home_directory();
        let config_file_path = PathBuf::from(&home_dir)
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
            .unwrap_or_else(|| Config::load_default_config(&home_dir)))
    }
}

impl Config {
    fn get_home_directory() -> String {
        std::env::var("HOME").expect("Fail to get home directory")
    }

    fn expand_home(&self) -> String {
        let home = Config::get_home_directory();
        let mut result = self.config_path.clone();
        if let Some(rest) = result.strip_prefix("${HOME}") {
            result = home.clone() + rest;
        }

        result
    }

    fn create_default(path: &std::path::Path) -> Result<(), IoError> {
        std::fs::create_dir_all(path.parent().unwrap_or(path))?;

        let content = include_str!("../config.toml");
        std::fs::write(path, content.as_bytes())?;

        log::info!("Default config created at {}", path.display());
        Ok(())
    }

    fn load_config(path: &PathBuf) -> Option<Config> {
        let text = std::fs::read_to_string(path).ok()?;
        let config: Config = toml::from_str(&text).ok()?;

        config.expand_home();

        log::info!("Config loaded from {}", path.display());
        Some(config)
    }

    fn load_default_config(base_dir: &str) -> Self {
        Config {
            config_path: format!("{}/.config/chronos_archive/config.toml", base_dir),
            db_path: format!("{}/.config/chronos_archive/config.toml", base_dir),
        }
    }
}
