use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    /// voice tone, traits, etc.
    pub context: String,
}

impl Candidate {
    pub fn new(name: String, context: String) -> Self {
        Self { name, context }
    }

    pub fn from_character(character: &Character, adventure_tone: Option<&str>) -> Self {
        Self {
            name: character.name.clone(),
            context: character.render(adventure_tone),
        }
    }

    pub fn from_adventure(adventure: &Adventure, candidates: Vec<String>) -> Vec<Self> {
        let mut parts = vec![];
        for character in adventure.characters.iter() {
            for candidate in candidates.iter() {
                if &character.name == candidate {
                    parts.push(Self::from_character(
                        character,
                        Some(adventure.tone.as_str()),
                    ));
                }
            }
        }

        parts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatReply {
    /// "narrator" or a character name.
    pub replier: String,
    pub reply: String,
}

#[derive(Error, Debug)]
pub enum ChatResponseError {
    #[error("the chat provider failed: {0}")]
    Provider(String),
}

pub trait ChatHandler: Send + Sync {
    fn character_respond(
        &self,
        candidate: &Candidate,
        memory: &str,
        user_message: &str,
    ) -> impl std::future::Future<Output = Result<ChatReply, ChatResponseError>> + Send;

    /// Let the model BOTH pick which character from `candidates` speaks and
    /// voice them. The narrator is never a choice here — that decision is made
    /// upstream by the director.
    fn autonomous_respond(
        &self,
        candidates: &[Candidate],
        memory: &str,
        user_message: &str,
    ) -> impl std::future::Future<Output = Result<ChatReply, ChatResponseError>> + Send;
}

pub trait NarratorHandler: Send + Sync {
    fn narrator_respond(
        &self,
        narrator_tone: &str,
        memory: &str,
        user_message: &str,
    ) -> impl std::future::Future<Output = Result<ChatReply, ChatResponseError>> + Send;
}
