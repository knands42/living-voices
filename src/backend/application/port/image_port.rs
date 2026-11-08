use std::future::Future;
use std::path::Path;

use thiserror::Error;

use crate::backend::domain::character::ImageFraming;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("the image provider failed: {0}")]
    Provider(String),
}

/// A shot already on disk, and the framing the new one should take against it.
#[derive(Clone, Copy, Debug)]
pub struct ShotReference<'a> {
    pub path: &'a Path,
    pub framing: ImageFraming,
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
    /// composed the layers and named the file, so the adapter only wires what it is given.
    fn queue_generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
        reference: Option<ShotReference<'_>>,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;

    fn generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
        reference: Option<ShotReference<'_>>,
    ) -> impl Future<Output = Result<(), ImageError>> + Send;
}
