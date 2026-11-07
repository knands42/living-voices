use serde::Serialize;

use crate::ai::integrations::text::open_api_client::OpenAiCompatibleClient;
use crate::ai::types::ChatMessage;
use crate::backend::application::port::chat_port::{ChatResponseError, NarratorPort, PromptTurn};
use crate::config::TextModel;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NarratorHandler {
    #[serde(skip)]
    config: TextModel,
}

impl NarratorHandler {
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

impl NarratorPort for NarratorHandler {
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
                super::keep_alive(&self.config),
            )
            .await
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        Ok(reply)
    }
}
