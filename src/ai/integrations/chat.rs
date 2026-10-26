use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::{
    client::{JsonSchema, OpenAiCompatibleClient, ResponseFormat},
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
pub struct ChatCompletion {
    pub replier: String,
    pub reply: String,
    #[serde(default)]
    pub act_closure: Option<u8>,
}

impl ChatCompletion {
    pub fn from_reply(raw: &str) -> Result<Self, ChatResponseError> {
        serde_json::from_str(raw).map_err(|e| ChatResponseError::Provider(e.to_string()))
    }
}

pub fn chat_reply_schema() -> JsonSchema {
    JsonSchema {
        name: "chat_reply".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "replier": { "type": "string" },
                "reply": { "type": "string" },
                "act_closure": { "type": ["integer", "null"], "minimum": 0, "maximum": 5 }
            },
            "required": ["replier", "reply", "act_closure"],
            "additionalProperties": false
        }),
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
