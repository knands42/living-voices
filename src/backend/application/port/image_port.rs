use std::future::Future;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("the image provider failed: {0}")]
    Provider(String),
}

pub trait ImagePort: Send + Sync {
    fn generate_scene(&self, prompt: &str) -> impl Future<Output = Result<(), ImageError>> + Send;

    /// One call for every character shot: the caller already decided the framing
    /// and composed the layers, so the adapter only queues what it is given.
    fn generate_character_image(
        &self,
        prompt: &str,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;
}
