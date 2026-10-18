use crate::ai::ChatMessage;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatResponseError {
    #[error("the chat provider failed: {0}")]
    Provider(String),
}

pub trait ChatHandler: Send + Sync {
    /// Voice a named character's next line. The application already chose the
    /// speaker, so the model returns only the line text.
    async fn character_respond(
        &self,
        character_tone: &str,
        history: &[ChatMessage],
        user_message: &str,
    ) -> Result<String, ChatResponseError>;

    /// Voice the narrator's next line.
    async fn narrator_respond(
        &self,
        narrator_tone: &str,
        history: &[ChatMessage],
        user_message: &str,
    ) -> Result<String, ChatResponseError>;
}
