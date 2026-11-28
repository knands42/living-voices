use env_logger::{Builder, Target};
use serde::{Deserialize, Serialize};
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

mod settings;

pub use settings::{
    DirectorSettings, ImageModel, MemorySettings, MessageThresholdRange, Models, Settings,
    SetupSettings, TextModel,
};

const CONFIG_FILE: &str = "config.toml";
const LOCAL_CONFIG_FILE: &str = "config-local.toml";
const DB_FILE: &str = "living_voices.db";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SupportedModel {
    Qwen,
    FluxQ4,
    FluxQ8,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub settings: Settings,
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub db_path: PathBuf,
}

/// One config for the whole app, readable at request time.
#[derive(Clone)]
pub struct ConfigHandle(Arc<RwLock<Config>>);

impl PartialEq for ConfigHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for ConfigHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConfigHandle")
    }
}

impl Default for ConfigHandle {
    fn default() -> Self {
        Self::new(Config {
            settings: Settings::default(),
            config_dir: PathBuf::new(),
            config_file: PathBuf::new(),
            db_path: PathBuf::new(),
        })
    }
}

impl ConfigHandle {
    pub fn new(config: Config) -> Self {
        Self(Arc::new(RwLock::new(config)))
    }

    pub fn load() -> Result<Self, IoError> {
        Config::new().map(Self::new)
    }

    pub fn settings(&self) -> Settings {
        self.read().settings.clone()
    }

    pub fn chat(&self) -> TextModel {
        self.read().settings.models.chat.clone()
    }

    pub fn memory(&self) -> TextModel {
        self.read().settings.models.memory.clone()
    }

    pub fn config_file(&self) -> PathBuf {
        self.read().config_file.clone()
    }

    pub fn config_dir(&self) -> PathBuf {
        self.read().config_dir.clone()
    }

    pub fn db_path(&self) -> PathBuf {
        self.read().db_path.clone()
    }

    pub fn needs_setup(&self) -> bool {
        self.read().needs_setup()
    }

    pub fn set_custom_config(&self, image_engine: bool) -> Result<(), IoError> {
        let mut config = self.write();
        config.set_custom_config(image_engine)
    }

    pub fn set_text_endpoints(&self, chat_base_url: &str, memory_base_url: &str) {
        let mut config = self.write();
        retarget_text_endpoint(&mut config.settings.models.chat, chat_base_url);
        retarget_text_endpoint(&mut config.settings.models.memory, memory_base_url);
    }

    pub fn set_chat_endpoint(&self, chat_base_url: &str) {
        retarget_text_endpoint(&mut self.write().settings.models.chat, chat_base_url);
    }

    pub fn set_memory_endpoint(&self, memory_base_url: &str) {
        retarget_text_endpoint(&mut self.write().settings.models.memory, memory_base_url);
    }

    pub fn set_media_endpoint(&self, media_url: &str) {
        retarget_image_endpoint(&mut self.write().settings.models.image, media_url);
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Config> {
        self.0.read().expect("the config lock is poisoned")
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Config> {
        self.0.write().expect("the config lock is poisoned")
    }
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
            Settings::write_template(&config_file)?;
        }

        let mut settings = load_settings(&config_dir, &config_file);
        if settings.models.image.output_dir.is_relative() {
            settings.models.image.output_dir = config_dir.join(&settings.models.image.output_dir);
        }
        log::info!("Config loaded from {}", config_file.display());
        let db_path = config_dir.join(DB_FILE);

        Ok(Self {
            settings,
            config_dir,
            config_file,
            db_path,
        })
    }

    pub fn needs_setup(&self) -> bool {
        !self.settings.setup.prompted_gate
    }

    pub fn set_custom_config(&mut self, image_engine: bool) -> Result<(), IoError> {
        self.settings.setup.prompted_gate = true;
        self.settings.models.image.enabled = image_engine;
        self.settings.update_file(&self.config_file)
    }

    pub fn init_logging() {
        let mut builder = Builder::from_env(env_logger::Env::default().default_filter_or("info"));

        #[cfg(debug_assertions)]
        builder.target(Target::Stdout);

        #[cfg(not(debug_assertions))]
        {
            let path = base_dir().join("living-voices.log");
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(file) = File::create(path) {
                builder.target(Target::Pipe(Box::new(file)));
            }
        }

        builder.init();
    }
}

fn retarget_text_endpoint(model: &mut TextModel, base_url: &str) {
    *model = model.clone().with_endpoint(base_url);
}

fn retarget_image_endpoint(model: &mut ImageModel, base_url: &str) {
    *model = model.clone().with_endpoint(base_url);
}

#[cfg_attr(not(debug_assertions), allow(unused_variables))]
fn load_settings(config_dir: &Path, config_file: &Path) -> Settings {
    #[cfg(debug_assertions)]
    let settings = Settings::load_with_override(config_file, &config_dir.join(LOCAL_CONFIG_FILE));
    #[cfg(not(debug_assertions))]
    let settings = Settings::load(config_file);

    settings
}

#[cfg(debug_assertions)]
pub fn base_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(not(debug_assertions))]
pub fn base_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("Fail to get home directory");
    PathBuf::from(home).join(".living-voices")
}
