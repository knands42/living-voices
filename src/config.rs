use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.toml";
const DB_FILE: &str = "living_voices.db";

const DEFAULT_CHAT_MODEL_URL: &str = "http://localhost:11434/v1";
const DEFAULT_CHAT_MEMORY_URL: &str = "http://localhost:11434/v1";
const DEFAULT_IMAGE_MODEL_URL: &str = "http://localhost:8188";

const DEFAULT_CHAT_TEMPERATURE: f32 = 0.5;
const DEFAULT_MEMORY_TEMPERATURE: f32 = 0.1;
const DEFAULT_REQUEST_TIMEOUT_SECS: u16 = 120;

const DEFAULT_COMPRESS_AFTER_THRESHOLD: u8 = 70;
const DEFAULT_COMPRESS_LAST_MESSAGES: u8 = 50;
const DEFAULT_SUMMARY_LIMIT: u8 = 5;
const DEFAULT_FACT_LIMIT: u8 = 5;

const DEFAULT_TARGETED_PICK_CHANCE: f32 = 0.8;
const DEFAULT_NARRATOR_CHANGE_IF_OOC: f32 = 0.2;
const DEFAULT_ACT_MAX_MESSAGES: u16 = 200;

const DEFAULT_NARRATOR_RAMP_MIN: u8 = 20;
const DEFAULT_NARRATOR_RAMP_MAX: u8 = 30;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub models: Models,
    pub memory: MemorySettings,
    pub director: DirectorSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Models {
    pub chat: TextModel,
    pub memory: TextModel,
    pub image: ImageModel,
}

impl Default for Models {
    fn default() -> Self {
        Self {
            chat: TextModel::chat_default(),
            memory: TextModel::memory_default(),
            image: ImageModel::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct TextModel {
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub request_timeout_secs: u16,
}

impl TextModel {
    pub fn chat_default() -> Self {
        Self {
            temperature: DEFAULT_CHAT_TEMPERATURE,
            ..Self::default()
        }
    }

    pub fn memory_default() -> Self {
        Self {
            temperature: DEFAULT_MEMORY_TEMPERATURE,
            ..Self::default()
        }
    }
}

impl Default for TextModel {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_CHAT_MODEL_URL.to_string(),
            model: String::new(),
            temperature: DEFAULT_CHAT_TEMPERATURE,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct MemorySettings {
    pub compress_after_threshold: u8,
    pub compress_last_messages: u8,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            compress_after_threshold: DEFAULT_COMPRESS_AFTER_THRESHOLD,
            compress_last_messages: DEFAULT_COMPRESS_LAST_MESSAGES,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct DirectorSettings {
    pub targeted_pick_chance: f32,
    pub narrator_change_if_ooc: f32,
    pub act_max_messages: u16,
    pub narrator_pick_chance_after_message_threshold_range: MessageThresholdRange,
}

impl Default for DirectorSettings {
    fn default() -> Self {
        Self {
            targeted_pick_chance: DEFAULT_TARGETED_PICK_CHANCE,
            narrator_change_if_ooc: DEFAULT_NARRATOR_CHANGE_IF_OOC,
            act_max_messages: DEFAULT_ACT_MAX_MESSAGES,
            narrator_pick_chance_after_message_threshold_range: MessageThresholdRange::default(),
        }
    }
}

/// An inclusive `min..=max` message-count window, read from TOML as
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(default)]
pub struct MessageThresholdRange {
    pub min: u8,
    pub max: u8,
}

impl Default for MessageThresholdRange {
    fn default() -> Self {
        Self {
            min: DEFAULT_NARRATOR_RAMP_MIN,
            max: DEFAULT_NARRATOR_RAMP_MAX,
        }
    }
}

impl MessageThresholdRange {
    pub fn normalized(self) -> (u8, u8) {
        (self.min, self.max.max(self.min))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct ImageModel {
    pub backend: String,
    pub base_url: String,
    pub workflow: String,
}

impl Default for ImageModel {
    fn default() -> Self {
        Self {
            backend: "comfyui".to_string(),
            base_url: DEFAULT_IMAGE_MODEL_URL.to_string(),
            workflow: String::new(),
        }
    }
}

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
    PathBuf::from(home).join(".living-voice")
}
