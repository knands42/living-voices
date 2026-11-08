use std::future::Future;
use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("the image provider failed: {0}")]
    Provider(String),
}

pub trait ImagePort: Send + Sync {
    /// `destination` is the prefix the provider writes below its own output
    /// directory, with no extension: the caller owns the naming.
    fn queue_generate_scene(
        &self,
        prompt: &str,
        destination: &str,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;

    /// One call for every character shot: the caller already decided the framing,
    /// composed the layers and named the file, so the adapter only queues what it is given.
    /// `reference` is a shot already on disk the new one has to stay faithful to.
    fn queue_generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
        reference: Option<&Path>,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;

    fn generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
        reference: Option<&Path>,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;
}
