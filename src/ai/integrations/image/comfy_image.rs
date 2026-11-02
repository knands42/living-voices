use serde::Serialize;

use crate::backend::application::port::image_port::{
    GeneratedImage, ImageError, ImageHandler, ScenePrompt,
};
use crate::config::ImageModel;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComfyImage {
    #[serde(skip)]
    config: ImageModel,
}

impl ComfyImage {
    pub fn new(config: ImageModel) -> Self {
        Self { config }
    }

    fn client(&self) -> Result<reqwest::Client, ImageError> {
        todo!("build a reqwest client for self.config.base_url")
    }

    async fn queue_prompt(&self, prompt: &ScenePrompt) -> Result<String, ImageError> {
        todo!("load self.config.workflow, inject the prompt text, POST /prompt, return prompt_id")
    }

    async fn await_image(&self, prompt_id: &str) -> Result<Vec<u8>, ImageError> {
        todo!("poll /history/{prompt_id} until done, then GET /view for the PNG bytes")
    }
}

impl ImageHandler for ComfyImage {
    async fn generate_scene(&self, prompt: &ScenePrompt) -> Result<GeneratedImage, ImageError> {
        let prompt_id = self.queue_prompt(prompt).await?;
        let bytes = self.await_image(&prompt_id).await?;

        Ok(GeneratedImage {
            adventure_id: prompt.adventure_id,
            bytes,
        })
    }
}
