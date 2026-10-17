//! Compress conversation history into durable facts plus a rolling summary.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::{
    client::{JsonSchema, OpenAiCompatibleClient},
    error::AiError,
    types::{ChatMessage, Sampling},
};

const SYSTEM_PROMPT: &str = "\
You compress a roleplay conversation into a memory record.
Reply with JSON only, containing exactly these two fields:
- \"facts\": an array of short, atomic, self-contained statements about people, \
places, plans, promises and changes. Keep names, numbers and identifiers verbatim.
- \"summary\": one concise paragraph covering the conversation so far.
Never invent details that are not in the conversation.";

/// Durable memory distilled from a stretch of conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryUpdate {
    pub facts: Vec<String>,
    pub summary: String,
}

impl MemoryUpdate {
    /// Decode the model's reply. The reply is schema-constrained (see [`schema`]),
    /// so it is a single JSON object and needs no salvaging.
    pub fn from_reply(reply: &str) -> Result<Self, AiError> {
        serde_json::from_str(reply).map_err(|_| AiError::MalformedMemory {
            reply: reply.to_string(),
        })
    }
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

/// The shape the memory model must produce.
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
