use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::memory::{ActStatus, Fact, Permanence, State};
use thiserror::Error;
use uuid::Uuid;

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

impl From<FactPermanence> for Permanence {
    fn from(value: FactPermanence) -> Self {
        match value {
            FactPermanence::Permanent => Permanence::Permanent,
            FactPermanence::Plot => Permanence::Plot,
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

impl DistilledFact {
    pub fn into_facts(adventure_id: Uuid, act_id: Uuid, facts: Vec<Self>) -> Vec<Fact> {
        facts
            .into_iter()
            .map(|fact| fact.into_fact(adventure_id, act_id))
            .collect()
    }

    fn into_fact(self, adventure_id: Uuid, act_id: Uuid) -> Fact {
        Fact::new(
            adventure_id,
            self.subject.clone(),
            self.fact.clone(),
            act_id,
            Permanence::from(self.permanence),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DistilledState {
    pub day: String,
    pub place: String,
    pub characters_present: Vec<String>,
    pub open_threads: Vec<String>,
    pub conditions: Vec<String>,
}

impl DistilledState {
    pub fn into_state(self, adventure_id: Uuid) -> State {
        State::new(adventure_id)
            .with_conditions(self.conditions.clone())
            .with_characters_present(self.characters_present.clone())
            .with_open_threads(self.open_threads.clone())
            .with_day(self.day.clone())
            .with_place(self.place.clone())
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedActStatus {
    Resolved,
    Failed,
    Bypassed,
}

impl From<ClosedActStatus> for ActStatus {
    fn from(value: ClosedActStatus) -> Self {
        match value {
            ClosedActStatus::Resolved => ActStatus::Resolved,
            ClosedActStatus::Failed => ActStatus::Failed,
            ClosedActStatus::Bypassed => ActStatus::Bypassed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedAct {
    pub index_line: String,
    pub digest: String,
    pub status: ClosedActStatus,
}

#[derive(Error, Debug)]
pub enum ClosedActError {
    #[error("the act closer failed: {0}")]
    Provider(String),
}

pub trait ActCloser: Send + Sync {
    fn close_act(
        &self,
        context: &[PromptTurn],
    ) -> impl Future<Output = Result<ClosedAct, ClosedActError>> + Send;
}
