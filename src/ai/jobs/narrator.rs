use serde::Serialize;

use crate::ai::{client::OpenAiCompatibleClient, types::ChatMessage};
use crate::backend::application::port::chat_port::{
    ChatResponseError, NarratorHandler, PromptTurn,
};
use crate::config::TextModel;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Narrator {
    #[serde(skip)]
    config: TextModel,
}

impl Narrator {
    pub fn new(config: TextModel) -> Self {
        Self { config }
    }

    fn client(&self) -> Result<OpenAiCompatibleClient, ChatResponseError> {
        OpenAiCompatibleClient::with_timeout(
            &self.config.base_url,
            std::time::Duration::from_secs(self.config.request_timeout_secs as u64),
        )
        .map_err(|e| ChatResponseError::Provider(e.to_string()))
    }
}

impl NarratorHandler for Narrator {
    async fn narrator_respond(&self, context: &[PromptTurn]) -> Result<String, ChatResponseError> {
        let client = self.client()?;

        let prompt = context
            .iter()
            .cloned()
            .map(ChatMessage::from)
            .collect::<Vec<ChatMessage>>();

        let reply = client
            .send(
                self.config.model.as_str(),
                &prompt,
                self.config.temperature,
                None,
            )
            .await
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        Ok(reply)
    }
}
