use env_logger::{Builder, Target};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

mod settings;

pub use settings::{
    DirectorSettings, ImageModel, MemorySettings, MessageThresholdRange, Models, Settings,
    SetupSettings, TextModel,
};

const CONFIG_FILE: &str = "config.toml";
const DB_FILE: &str = "living_voices.db";

// TODO: Use another one for the ai integration and provision module called Workflow, these ones only lists all available models, the other one will dictate what to install from a specific workflow (e.g., flux workflow automatically install kontext)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SupportedModel {
    Qwen,
    FluxQ4,
    FluxQ8,
    FluxKontextQ4,
    FluxKontextQ8,
}

impl SupportedModel {
    pub fn is_reference_model(self) -> bool {
        matches!(self, Self::FluxKontextQ4 | Self::FluxKontextQ8)
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
            Settings::write_template(&config_file)?;
        }

        let mut settings = Settings::load(&config_file);
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

    pub fn set_media_engine(&mut self, enabled: bool) -> Result<(), IoError> {
        self.settings.models.image.enabled = enabled;
        self.settings.setup.prompted_gate = true;
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

    /// Route chat and memory at their own endpoints, one engine each.
    pub fn with_text_endpoints(mut self, chat_base_url: &str, memory_base_url: &str) -> Self {
        self.settings.models.chat = self.settings.models.chat.with_endpoint(chat_base_url);
        self.settings.models.memory = self.settings.models.memory.with_endpoint(memory_base_url);
        self
    }

    /// Route the image backend at the provisioned ComfyUI `base_url`.
    pub fn with_media_engine(mut self, base_url: &str) -> Self {
        self.settings.models.image.base_url = base_url.trim_end_matches('/').to_string();
        self
    }

    pub fn persist_to_file(&self) -> Result<(), std::io::Error> {
        self.settings.update_file(self.config_file.as_path())?;
        Ok(())
    }
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
