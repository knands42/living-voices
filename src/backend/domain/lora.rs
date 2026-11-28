use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoraKind {
    #[default]
    Text,
    Image,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lora {
    /// File name under the engine's adapter directory, never an absolute path.
    #[serde(alias = "path")]
    pub name: String,
    pub scale: f32,
    #[serde(default)]
    pub kind: LoraKind,
    #[serde(skip)]
    pub source: Option<PathBuf>, // TODO: Remove
}

impl Lora {
    pub const DEFAULT_SCALE: f32 = 1.0;

    pub fn text(name: impl Into<String>, scale: f32) -> Self {
        Self {
            name: name.into(),
            scale,
            kind: LoraKind::Text,
            source: None,
        }
    }

    pub fn image(name: impl Into<String>, scale: f32) -> Self {
        Self {
            name: name.into(),
            scale,
            kind: LoraKind::Image,
            source: None,
        }
    }

    /// A file the author picked: the name is its file name, the source is kept only so the manager
    /// can copy it into the engine's adapter directory before saving.
    pub fn from_source(source: impl AsRef<Path>, scale: f32, kind: LoraKind) -> Self {
        let source = source.as_ref();
        let name = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        Self {
            name,
            scale,
            kind,
            source: Some(source.to_path_buf()),
        }
    }
}
