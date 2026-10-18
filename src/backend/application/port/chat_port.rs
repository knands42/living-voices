use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use thiserror::Error;

/// One candidate the autonomous call may pick, with the tone it should use.
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

    pub fn from_adventure(adventure: &Adventure) -> Vec<Self> {
        let mut parts = vec![];
        for character in adventure.characters.iter() {
            parts.push(Self::from_character(
                character,
                Some(adventure.tone.as_str()),
            ));
        }

        parts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatReply {
    /// "narrator" or a character name.
    pub replier: String,
    /// The narration or dialogue text.
    pub reply: String,
}

#[derive(Error, Debug)]
pub enum ChatResponseError {
    #[error("the chat provider failed: {0}")]
    Provider(String),
}

pub trait ChatHandler: Send + Sync {
    /// Voice a named character's next line
    async fn character_respond(
        &self,
        candidate: &Candidate,
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError>;

    /// Voice the narrator's next line
    async fn narrator_respond(
        &self,
        narrator_tone: &str,
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError>;

    /// Let the model BOTH pick the speaker (from `cast`) and voice them
    async fn autonomous_respond(
        &self,
        narrator_tone: &str,
        candidates: &[Candidate],
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError>;
}
