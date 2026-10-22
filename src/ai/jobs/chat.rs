use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::{
    client::{JsonSchema, OpenAiCompatibleClient, ResponseFormat},
    error::AiError,
    types::ChatMessage,
};
use crate::backend::application::port::chat_port::{
    Candidate, ChatHandler, ChatReply, ChatResponseError,
};
use crate::config::TextModel;

const CHARACTER_SYSTEM_PROMPT: &str = "\
You are a single character in a roleplay. Stay fully in character — speak only as
them, in their voice, revealing only what they would know. Reply with their next
line of dialogue or action as plain text — no narration of other characters, no
JSON, no quotation marks around the whole reply.";

const AUTONOMOUS_SYSTEM_PROMPT: &str = "\
You direct one turn of a roleplay. From the cast listed below, choose which
character speaks next — never the narrator — then write their line in their own
voice and tone.
Reply with JSON only, exactly these two fields:
- \"replier\": the exact name of the character you chose from the cast.
- \"reply\": that character's line, as plain text.";

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
            std::time::Duration::from_secs(self.config.request_timeout_secs),
        )
        .map_err(|e| ChatResponseError::Provider(e.to_string()))
    }
}

impl ChatHandler for Chat {
    async fn character_respond(
        &self,
        candidate: &Candidate,
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError> {
        let client = self.client()?;

        let messages = [
            ChatMessage::system(CHARACTER_SYSTEM_PROMPT),
            ChatMessage::system(candidate.context.as_str()),
            ChatMessage::system(memory),
            ChatMessage::user(user_message),
        ];

        // TODO: add keep_alive for caching tokens
        let reply = client
            .send(
                self.config.model.as_str(),
                &messages,
                self.config.temperature,
                None,
            )
            .await
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        Ok(ChatReply {
            replier: candidate.name.clone(),
            reply,
        })
    }

    async fn autonomous_respond(
        &self,
        candidates: &[Candidate],
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError> {
        let client = self.client()?;

        let messages = [
            ChatMessage::system(AUTONOMOUS_SYSTEM_PROMPT),
            ChatMessage::system(
                candidates
                    .iter()
                    .map(|c| format!("- \"{}\": {}", c.name, c.context))
                    .collect::<Vec<String>>()
                    .join("\n"),
            ),
            ChatMessage::system(memory),
            ChatMessage::user(user_message),
        ];

        let format = ResponseFormat::JsonSchema {
            json_schema: &schema(),
        };

        let raw = client
            .send(
                self.config.model.as_str(),
                &messages,
                self.config.temperature,
                Some(format),
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

/// JSON schema forcing `{ replier, reply }` for the autonomous call.
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
