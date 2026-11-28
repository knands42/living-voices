use serde::{Deserialize, Serialize};
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use super::{SupportedModel, base_dir};

const DEFAULT_CHAT_MODEL_URL: &str = "http://localhost:11434/v1";
const DEFAULT_CHAT_MEMORY_URL: &str = "http://localhost:11434/v1";
const DEFAULT_IMAGE_MODEL_URL: &str = "http://localhost:8188";
const DEFAULT_IMAGE_OUTPUT_DIR: &str = "comfyui/output";

const DEFAULT_CHAT_TEMPERATURE: f32 = 0.5;
const DEFAULT_MEMORY_TEMPERATURE: f32 = 0.1;
const DEFAULT_REQUEST_TIMEOUT_SECS: u16 = 120;
const DEFAULT_KEEP_ALIVE: &str = "5m";

const DEFAULT_COMPRESS_AFTER_THRESHOLD: u8 = 70;
const DEFAULT_COMPRESS_LAST_MESSAGES: u8 = 50;

const DEFAULT_TARGETED_PICK_CHANCE: f32 = 0.8;
const DEFAULT_NARRATOR_CHANGE_IF_OOC: f32 = 0.2;
const DEFAULT_ACT_MAX_MESSAGES: u16 = 200;

const DEFAULT_FOLLOW_UP_CHANCE: f32 = 0.35;
const DEFAULT_FOLLOW_UP_MAX_CHAIN: u8 = 6;

const DEFAULT_NARRATOR_RAMP_MIN: u8 = 20;
const DEFAULT_NARRATOR_RAMP_MAX: u8 = 30;

const DEFAULT_NARRATOR_MIN_GAP: u8 = 10;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub models: Models,
    pub memory: MemorySettings,
    pub director: DirectorSettings,
    pub setup: SetupSettings,
}

/// The only state the first-run gate owns. Every other gate answer is written
/// straight into the setting it belongs to.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct SetupSettings {
    pub prompted_gate: bool,
}

impl SetupSettings {
    pub fn prompt_gate_setup(&mut self) {
        self.prompted_gate = true;
    }
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
    /// Fallback endpoint. A provisioned text engine replaces it through
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub request_timeout_secs: u16,
    /// Ollama's `keep_alive`, e.g. "5m", "1h", "0" to unload, "-1" to keep forever.
    pub keep_alive: String,
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
            base_url: DEFAULT_CHAT_MEMORY_URL.to_string(),
            ..Self::default()
        }
    }

    pub fn with_endpoint(mut self, base_url: &str) -> Self {
        self.base_url = format!("{}/v1", base_url.trim_end_matches('/'));
        self
    }
}

impl Default for TextModel {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_CHAT_MODEL_URL.to_string(),
            model: String::new(),
            temperature: DEFAULT_CHAT_TEMPERATURE,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
            keep_alive: DEFAULT_KEEP_ALIVE.to_string(),
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
    /// chance that an NPC follows another NPC's reply, before the per-step decay
    pub follow_up_chance: f32,
    /// hard cap on NPC-to-NPC follow-ups chained off one player beat
    pub follow_up_max_chain: u8,
    /// messages that must pass after a narrator turn before it may speak again
    pub narrator_min_gap: u8,
    pub narrator_pick_chance_after_message_threshold_range: MessageThresholdRange,
}

impl Default for DirectorSettings {
    fn default() -> Self {
        Self {
            targeted_pick_chance: DEFAULT_TARGETED_PICK_CHANCE,
            narrator_change_if_ooc: DEFAULT_NARRATOR_CHANGE_IF_OOC,
            act_max_messages: DEFAULT_ACT_MAX_MESSAGES,
            follow_up_chance: DEFAULT_FOLLOW_UP_CHANCE,
            follow_up_max_chain: DEFAULT_FOLLOW_UP_MAX_CHAIN,
            narrator_min_gap: DEFAULT_NARRATOR_MIN_GAP,
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
    pub enabled: bool,
    pub backend: String,
    pub base_url: String,
    pub workflow: SupportedModel,
    pub output_dir: PathBuf,
}

impl Default for ImageModel {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "comfyui".to_string(),
            base_url: DEFAULT_IMAGE_MODEL_URL.to_string(),
            workflow: SupportedModel::FluxQ8,
            output_dir: base_dir().join(DEFAULT_IMAGE_OUTPUT_DIR),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        Self::load_layered(&[path])
    }

    /// Debug builds layer `config-local.toml` over `config.toml`, so the local
    /// file only has to carry the keys it changes.
    #[cfg(debug_assertions)]
    pub fn load_with_override(base: &Path, overrides: &Path) -> Self {
        if !overrides.exists() {
            return Self::load(base);
        }

        log::info!("Config overrides from {}", overrides.display());
        Self::load_layered(&[base, overrides])
    }

    fn load_layered(paths: &[&Path]) -> Self {
        let mut merged = toml::Value::Table(toml::Table::new());

        for path in paths {
            match Self::read_layer(path) {
                Ok(value) => merge(&mut merged, value),
                Err(err) => log::warn!("Ignoring {err}"),
            }
        }

        merged.try_into().unwrap_or_else(|err| {
            log::warn!("Invalid config: {err}");
            Self::default()
        })
    }

    fn read_layer(path: &Path) -> Result<toml::Value, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|err| format!("config at {}: {err}", path.display()))?;
        let value: toml::Value =
            toml::from_str(&text).map_err(|err| format!("config at {}: {err}", path.display()))?;

        // A layer with a wrong type must not poison the layers under it.
        value
            .clone()
            .try_into::<Settings>()
            .map_err(|err| format!("config at {}: {err}", path.display()))?;

        Ok(value)
    }

    /// The commented template shipped with the binary, for a machine that has
    /// never run before.
    pub fn write_template(path: &Path) -> Result<(), IoError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, include_str!("../../config.toml").as_bytes())?;

        log::info!("Default config created at {}", path.display());
        Ok(())
    }

    pub fn update_file(&self, path: &Path) -> Result<(), IoError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let body = toml::to_string_pretty(self).map_err(IoError::other)?;
        std::fs::write(path, body)?;
        log::info!("Config written to {}", path.display());

        Ok(())
    }
}

fn merge(base: &mut toml::Value, overrides: toml::Value) {
    match overrides {
        toml::Value::Table(override_table) => match base {
            toml::Value::Table(base_table) => {
                for (key, value) in override_table {
                    match base_table.get_mut(&key) {
                        Some(slot) => merge(slot, value),
                        None => {
                            base_table.insert(key, value);
                        }
                    }
                }
            }
            slot => *slot = toml::Value::Table(override_table),
        },
        other => *base = other,
    }
}
