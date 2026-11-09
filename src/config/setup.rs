use std::io::Error as IoError;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{ImageModel, SupportedModel};

/// What the first-run gate answered. It mirrors the `config.toml` shape so the two
/// files read the same way, but only the keys the gate owns appear here.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Setup {
    pub models: SetupModels,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(default)]
pub struct SetupModels {
    pub image: Option<ImageModelChoice>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(default)]
pub struct ImageModelChoice {
    pub enabled: bool,
    pub workflow: SupportedModel,
}

impl Default for ImageModelChoice {
    fn default() -> Self {
        Self {
            enabled: false,
            workflow: SupportedModel::FluxQ4,
        }
    }
}

impl ImageModelChoice {
    pub fn apply_to(self, image: &mut ImageModel) {
        image.enabled = self.enabled;
        image.workflow = self.workflow;
    }
}

impl Setup {
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(setup) => setup,
                Err(err) => {
                    log::warn!("Invalid setup at {}: {err}", path.display());
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), IoError> {
        let body = toml::to_string_pretty(self).map_err(IoError::other)?;

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)?;
        log::info!("Setup answer saved to {}", path.display());

        Ok(())
    }

    /// `None` means the gate has never answered on this machine.
    pub fn media_choice(&self) -> Option<ImageModelChoice> {
        self.models.image
    }

    pub fn needs_answer(&self) -> bool {
        self.models.image.is_none()
    }
}
