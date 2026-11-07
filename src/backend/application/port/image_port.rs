use std::future::Future;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("the image provider failed: {0}")]
    Provider(String),
}

pub trait ImagePort: Send + Sync {
    /// `destination` is the prefix the provider writes below its own output
    /// directory, with no extension: the caller owns the naming.
    fn generate_scene(
        &self,
        prompt: &str,
        destination: &str,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;

    /// One call for every character shot: the caller already decided the framing,
    /// composed the layers and named the file, so the adapter only queues what it is given.
    fn generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;
}
