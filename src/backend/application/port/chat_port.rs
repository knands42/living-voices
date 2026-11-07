use std::future::Future;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    System,
    Player,
    Cast,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTurn {
    pub speaker: Speaker,
    pub text: String,
}

impl PromptTurn {
    pub fn system(text: impl Into<String>) -> Self {
        Self::new(Speaker::System, text)
    }

    pub fn player(text: impl Into<String>) -> Self {
        Self::new(Speaker::Player, text)
    }

    pub fn cast(text: impl Into<String>) -> Self {
        Self::new(Speaker::Cast, text)
    }

    fn new(speaker: Speaker, text: impl Into<String>) -> Self {
        Self {
            speaker,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatReply {
    pub replier: String,
    pub reply: String,
    pub act_closure: Option<u8>,
}

#[derive(Error, Debug)]
pub enum ChatResponseError {
    #[error("the chat provider failed: {0}")]
    Provider(String),
}

pub trait ChatHandler: Send + Sync {
    fn character_respond(
        &self,
        context: &[PromptTurn],
    ) -> impl Future<Output = Result<ChatReply, ChatResponseError>> + Send;

    fn autonomous_respond(
        &self,
        context: &[PromptTurn],
    ) -> impl Future<Output = Result<ChatReply, ChatResponseError>> + Send;

    /// One NPC-to-NPC follow-up beat. The model picks which present cast member
    /// speaks; `act_closure` is always `None` since a follow-up never judges closure.
    fn follow_up_respond(
        &self,
        context: &[PromptTurn],
    ) -> impl Future<Output = Result<ChatReply, ChatResponseError>> + Send;
}

pub trait NarratorHandler: Send + Sync {
    fn narrator_respond(
        &self,
        context: &[PromptTurn],
    ) -> impl Future<Output = Result<String, ChatResponseError>> + Send;
}

pub trait PromptHandler: Send + Sync {
    fn handle_prompt(
        &self,
        context: &[PromptTurn],
    ) -> impl Future<Output = Result<String, ChatResponseError>> + Send;
}
