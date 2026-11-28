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
    pub name: String,
    pub scale: f32,
    #[serde(default)]
    pub kind: LoraKind,
}

impl Lora {
    pub const DEFAULT_SCALE: f32 = 1.0;

    pub fn text(name: impl Into<String>, scale: f32) -> Self {
        Self {
            name: name.into(),
            scale,
            kind: LoraKind::Text,
        }
    }

    pub fn image(name: impl Into<String>, scale: f32) -> Self {
        Self {
            name: name.into(),
            scale,
            kind: LoraKind::Image,
        }
    }
}
