use thiserror::Error;

use crate::backend::domain::message::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistilledMemory {
    pub facts: Vec<String>,
    pub summary: String,
}

#[derive(Error, Debug)]
pub enum DistillError {
    #[error("the memory provider failed: {0}")]
    Provider(String),
}

pub trait MemoryDistiller: Send + Sync {
    /// Distill a stretch of conversation into facts plus a rolling summary.
    async fn distill(&self, messages: &[Message]) -> Result<DistilledMemory, DistillError>;
}
