use std::future::Future;

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePrompt {
    pub adventure_id: Uuid,
    pub title: String,
    pub headline: String,
    pub tone: String,
}

impl ScenePrompt {
    pub fn new(
        adventure_id: Uuid,
        title: impl Into<String>,
        headline: impl Into<String>,
        tone: impl Into<String>,
    ) -> Self {
        Self {
            adventure_id,
            title: title.into(),
            headline: headline.into(),
            tone: tone.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedImage {
    pub adventure_id: Uuid,
    pub bytes: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("the image provider failed: {0}")]
    Provider(String),
}

pub trait ImageHandler: Send + Sync {
    fn generate_scene(
        &self,
        prompt: &ScenePrompt,
    ) -> impl Future<Output = Result<GeneratedImage, ImageError>> + Send;
}
