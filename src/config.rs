use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.toml";
const DB_FILE: &str = "living_voices.db";

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {}

#[derive(Debug, Clone)]
pub struct Config {
    pub settings: Settings,
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub db_path: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self, IoError> {
        let config_dir = base_dir().to_path_buf();
        let config_file = config_dir.join(CONFIG_FILE);

        if !config_file.exists() {
            log::info!(
                "Config setup being initialized at {}",
                config_file.display()
            );
            Self::create_config_file(&config_file)?;
        }

        let settings = Self::load_config_into_file(&config_file);

        log::info!("Config loaded from {}", config_file.display());

        let db_path = config_dir.join(DB_FILE);

        Ok(Self {
            settings,
            config_dir,
            config_file,
            db_path,
        })
    }

    fn load_config_into_file(path: &Path) -> Settings {
        match std::fs::read_to_string(path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(settings) => settings,
                Err(err) => {
                    log::warn!("Invalid config at {}: {err}", path.display());
                    Settings::default()
                }
            },
            Err(err) => {
                log::warn!("Could not read config at {}: {err}", path.display());
                Settings::default()
            }
        }
    }

    fn create_config_file(path: &Path) -> Result<(), IoError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, include_str!("../config.toml").as_bytes())?;

        log::info!("Default config created at {}", path.display());
        Ok(())
    }
}

#[cfg(debug_assertions)]
fn base_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(not(debug_assertions))]
fn base_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("Fail to get home directory");
    PathBuf::from(home).join("projects").join("living-voice")
}
