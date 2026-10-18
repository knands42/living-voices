use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.toml";
const DB_FILE: &str = "living_voices.db";

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434/v1";
const DEFAULT_COMFYUI_URL: &str = "http://localhost:8188";

/// Character dialogue: expressive but coherent.
const DEFAULT_CHAT_TEMPERATURE: f32 = 0.5;
/// Memory/summary extraction: faithful, no invention.
const DEFAULT_MEMORY_TEMPERATURE: f32 = 0.1;
/// Seconds before an in-flight model request is abandoned.
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 120;

/// Distill once this many messages are uncovered by any summary.
const DEFAULT_COMPRESS_THRESHOLD: u64 = 80;
/// How many summaries the prompt-context block carries.
const DEFAULT_SUMMARY_LIMIT: u64 = 5;
/// How many facts the prompt-context block carries.
const DEFAULT_FACT_LIMIT: u64 = 5;

/// Chance (0.0–1.0) that an explicitly targeted character answers directly,
/// rather than deferring the pick to the autonomous model call.
const DEFAULT_TARGETED_PICK_CHANCE: f32 = 0.8;

const DEFAULT_NARRATOR_CHANGE_IF_OOC: f32 = 0.2;

/// Below `min` messages since the narrator last spoke, the ramp adds nothing to
/// the narrator's chance; from `min` to `max` it ramps 0→100%; at/after `max`
/// the narrator is certain to take the turn.
const DEFAULT_NARRATOR_RAMP_MIN: u64 = 20;
const DEFAULT_NARRATOR_RAMP_MAX: u64 = 30;

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
    pub request_timeout_secs: u64,
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
            base_url: DEFAULT_OLLAMA_URL.to_string(),
            model: String::new(),
            temperature: DEFAULT_CHAT_TEMPERATURE,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct MemorySettings {
    /// Trigger distillation once this many messages are uncovered by any summary.
    pub compress_threshold: u64,
    /// How many recent summaries the prompt-context block carries.
    pub summary_limit: u64,
    /// How many recent facts the prompt-context block carries.
    pub fact_limit: u64,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            compress_threshold: DEFAULT_COMPRESS_THRESHOLD,
            summary_limit: DEFAULT_SUMMARY_LIMIT,
            fact_limit: DEFAULT_FACT_LIMIT,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct DirectorSettings {
    /// Chance (0.0–1.0)
    pub targeted_pick_chance: f32,
    /// Chance (0.0–1.0)
    pub narrator_change_if_ooc: f32,
    /// As messages pile up without the narrator speaking, its chance to take
    pub narrator_pick_chance_after_message_threshold_range: MessageThresholdRange,
}

impl Default for DirectorSettings {
    fn default() -> Self {
        Self {
            targeted_pick_chance: DEFAULT_TARGETED_PICK_CHANCE,
            narrator_change_if_ooc: DEFAULT_NARRATOR_CHANGE_IF_OOC,
            narrator_pick_chance_after_message_threshold_range: MessageThresholdRange::default(),
        }
    }
}

/// An inclusive `min..=max` message-count window, read from TOML as
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(default)]
pub struct MessageThresholdRange {
    pub min: u64,
    pub max: u64,
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
    pub fn normalized(self) -> (u64, u64) {
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
            base_url: DEFAULT_COMFYUI_URL.to_string(),
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
    PathBuf::from(home).join("projects").join("living-voice")
}
