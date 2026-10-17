pub mod client;
pub mod error;
pub mod jobs;
pub mod types;

pub use client::OpenAiCompatibleClient;
pub use error::AiError;
pub use types::{ChatMessage, Role, Sampling};
