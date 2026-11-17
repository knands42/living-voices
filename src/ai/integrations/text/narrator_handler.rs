use serde::Serialize;

use crate::ai::integrations::text::chat_handler::{ChatCompletion, chat_reply_schema};
use crate::ai::integrations::text::open_api_client::{OpenAiCompatibleClient, ResponseFormat};
use crate::ai::types::ChatMessage;
use crate::backend::application::port::chat_port::{
    ChatReply, ChatResponseError, NarratorPort, PromptTurn,
};
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

    async fn narrator_chat_respond(
        &self,
        context: &[PromptTurn],
    ) -> Result<ChatReply, ChatResponseError> {
        let client = self.client()?;

        let prompt = context
            .iter()
            .cloned()
            .map(ChatMessage::from)
            .collect::<Vec<ChatMessage>>();

        let format = ResponseFormat::JsonSchema {
            json_schema: &chat_reply_schema(),
        };

        let raw = client
            .send(
                self.config.model.as_str(),
                &prompt,
                self.config.temperature,
                Some(format),
                super::keep_alive(&self.config),
            )
            .await
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        let completion = ChatCompletion::from_reply(&raw)?;

        Ok(ChatReply {
            replier: completion.replier,
            reply: completion.reply,
            act_closure: completion.act_closure,
        })
    }
}
