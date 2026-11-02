use std::future::Future;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("the image provider failed: {0}")]
    Provider(String),
}

pub trait ImageHandler: Send + Sync {
    fn generate_scene(&self, prompt: &str) -> impl Future<Output = Result<(), ImageError>> + Send;

    fn generate_character_portrait(
        &self,
        prompt: &str,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;

    fn generate_character_image(
        &self,
        prompt: &str,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;
}
