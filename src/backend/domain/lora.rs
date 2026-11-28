use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lora {
    pub path: String,
    pub scale: f32,
}

impl Lora {
    pub const DEFAULT_SCALE: f32 = 1.0;

    pub fn new(path: impl Into<String>, scale: f32) -> Self {
        Self {
            path: path.into(),
            scale,
        }
    }
}
