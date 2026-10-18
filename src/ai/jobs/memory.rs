use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::{
    client::{JsonSchema, OpenAiCompatibleClient},
    error::AiError,
    types::{ChatMessage, Sampling},
};
use crate::backend::application::port::memory_port::{
    DistillError, DistilledMemory, MemoryDistiller,
};
use crate::backend::domain::message::Message;

const SYSTEM_PROMPT: &str = "\
You compress a roleplay conversation into a memory record.
Reply with JSON only, containing exactly these two fields:
- \"facts\": an array of short, atomic, self-contained statements about people, \
places, plans, promises and changes. Keep names, numbers and identifiers verbatim.
- \"summary\": one concise paragraph covering the conversation so far (in order).
Never invent details that are not in the conversation.";

/// `base_url`/`model` are distiller config, not part of the model's JSON reply (serde skips them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryUpdate {
    pub facts: Vec<String>,
    pub summary: String,
    #[serde(skip)]
    base_url: String,
    #[serde(skip)]
    model: String,
}

impl MemoryUpdate {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            facts: vec![],
            summary: String::new(),
            base_url,
            model,
        }
    }

    pub fn from_reply(reply: &str) -> Result<Self, AiError> {
        serde_json::from_str(reply).map_err(|_| AiError::MalformedMemory {
            reply: reply.to_string(),
        })
    }

    /// Distill `history` into facts and a summary using the memory model.
    pub async fn distill(
        client: &OpenAiCompatibleClient,
        model: &str,
        history: &[ChatMessage],
    ) -> Result<MemoryUpdate, AiError> {
        let mut messages = Vec::with_capacity(history.len() + 1);
        messages.push(ChatMessage::system(SYSTEM_PROMPT));
        messages.extend_from_slice(history);

        let format = crate::ai::client::ResponseFormat::JsonSchema {
            json_schema: &schema(),
        };

        let reply = client
            .send(model, &messages, Sampling::EXTRACTION, Some(format))
            .await?;

        MemoryUpdate::from_reply(&reply)
    }
}

impl MemoryDistiller for MemoryUpdate {
    async fn distill(&self, messages: &[Message]) -> Result<DistilledMemory, DistillError> {
        let history: Vec<ChatMessage> = messages.iter().map(ChatMessage::from).collect();
        // TODO: Consider adding client as part of the struct
        let client = OpenAiCompatibleClient::new(&self.base_url)
            .map_err(|e| DistillError::Provider(e.to_string()))?;
        let update = MemoryUpdate::distill(&client, &self.model, &history)
            .await
            .map_err(|e| DistillError::Provider(e.to_string()))?;

        Ok(DistilledMemory::from(update))
    }
}

impl From<MemoryUpdate> for DistilledMemory {
    fn from(memory_update: MemoryUpdate) -> Self {
        DistilledMemory {
            summary: memory_update.summary,
            facts: memory_update.facts,
        }
    }
}

fn schema() -> JsonSchema {
    JsonSchema {
        name: "memory_update".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "facts": { "type": "array", "items": { "type": "string" } },
                "summary": { "type": "string" }
            },
            "required": ["facts", "summary"],
            "additionalProperties": false
        }),
    }
}
