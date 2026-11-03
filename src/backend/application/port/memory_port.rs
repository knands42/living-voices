use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::memory::{ActStatus, CharacterCondition, Fact, Permanence, State};
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
    pub fn into_facts(facts: Vec<Self>) -> Vec<Fact> {
        facts.into_iter().map(Self::into_fact).collect()
    }

    fn into_fact(self) -> Fact {
        Fact::new(self.subject, self.fact, Permanence::from(self.permanence))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DistilledState {
    pub day: String,
    pub place: String,
    pub characters_present: Vec<String>,
    pub open_threads: Vec<String>,
    pub character_conditions: Vec<CharacterCondition>,
}

impl DistilledState {
    /// Build the live `State`. Removing leftovers from the LLM where the player is listed among the NPCs.
    pub fn into_state(self, adventure_id: Uuid, player_name: Option<&str>) -> State {
        let characters_present = self
            .characters_present
            .into_iter()
            .filter(|name| match player_name {
                Some(player) => !name.eq_ignore_ascii_case(player),
                None => true,
            })
            .collect();

        State::new(Uuid::new_v4(), adventure_id)
            .with_character_conditions(self.character_conditions)
            .with_characters_present(characters_present)
            .with_open_threads(self.open_threads)
            .with_day(self.day)
            .with_place(self.place)
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

#[cfg(test)]
mod into_state_tests {
    use super::*;

    fn distilled_with_present(names: &[&str]) -> DistilledState {
        DistilledState {
            characters_present: names.iter().map(|n| n.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn drops_the_player_from_characters_present() {
        let distilled = distilled_with_present(&["You", "Mira", "Kael"]);

        let state = distilled.into_state(Uuid::new_v4(), Some("You"));

        assert_eq!(
            state.characters_present,
            vec!["Mira".to_string(), "Kael".to_string()]
        );
    }

    #[test]
    fn drops_the_player_case_insensitively() {
        let distilled = distilled_with_present(&["you", "Mira"]);

        let state = distilled.into_state(Uuid::new_v4(), Some("You"));

        assert_eq!(state.characters_present, vec!["Mira".to_string()]);
    }

    #[test]
    fn keeps_everyone_when_there_is_no_player() {
        let distilled = distilled_with_present(&["Mira", "Kael"]);

        let state = distilled.into_state(Uuid::new_v4(), None);

        assert_eq!(
            state.characters_present,
            vec!["Mira".to_string(), "Kael".to_string()]
        );
    }
}
