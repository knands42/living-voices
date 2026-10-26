use crate::backend::{application::port::chat_port::PromptTurn, domain::memory::Fact};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactPermanence {
    Permanent,
    Plot,
}

impl FactPermanence {
    pub fn as_str(self) -> &'static str {
        match self {
            FactPermanence::Permanent => "permanent",
            FactPermanence::Plot => "plot",
        }
    }
}

/// One atomic fact destined for a `facts` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistilledFact {
    pub subject: String,
    pub fact: String,
    pub permanence: FactPermanence,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DistilledState {
    pub day: String,
    pub place: String,
    pub characters_present: Vec<String>,
    pub open_threads: Vec<String>,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistilledMemory {
    pub facts: Vec<DistilledFact>,
    pub state: DistilledState,
    pub running_digest: String,
}

#[derive(Error, Debug)]
pub enum DistillError {
    #[error("the memory provider failed: {0}")]
    Provider(String),
}

pub trait MemoryDistiller: Send + Sync {
    fn distill(
        &self,
        context: &[PromptTurn],
    ) -> impl Future<Output = Result<DistilledMemory, DistillError>> + Send;
}
