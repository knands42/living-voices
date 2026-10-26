use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::{
    client::{JsonSchema, OpenAiCompatibleClient, ResponseFormat},
    error::AiError,
    types::ChatMessage,
};
use crate::backend::application::port::chat_port::{
    ChatHandler, ChatReply, ChatResponseError, PromptTurn,
};
use crate::config::TextModel;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chat {
    #[serde(skip)]
    config: TextModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ChatCompletion {
    replier: String,
    reply: String,
}

impl ChatCompletion {
    fn from_reply(raw: &str) -> Result<Self, AiError> {
        serde_json::from_str(raw).map_err(|_| AiError::MalformedMemory {
            reply: raw.to_string(),
        })
    }
}

impl Chat {
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

impl ChatHandler for Chat {
    async fn character_respond(&self, context: &[PromptTurn]) -> Result<String, ChatResponseError> {
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

    async fn autonomous_respond(
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
            json_schema: &schema(),
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

        let completion = ChatCompletion::from_reply(&raw)
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        Ok(ChatReply {
            replier: completion.replier,
            reply: completion.reply,
        })
    }
}

fn schema() -> JsonSchema {
    JsonSchema {
        name: "chat_reply".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "replier": { "type": "string" },
                "reply": { "type": "string" }
            },
            "required": ["replier", "reply"],
            "additionalProperties": false
        }),
    }
}
