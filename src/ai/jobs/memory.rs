//! Compress conversation history into durable facts plus a rolling summary.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai::{
    client::OpenAiCompatibleClient,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryUpdate {
    pub facts: Vec<String>,
    pub summary: String,
}

impl MemoryUpdate {
    /// Parse a model reply, tolerating ```json fences, surrounding prose, and a
    /// reply split across several JSON objects (the distiller tends to emit
    /// `facts` and `summary` as separate objects). Objects are merged.
    pub fn from_reply(reply: &str) -> Result<Self, AiError> {
        let mut facts = Vec::new();
        let mut summary = None;

        for object in json_objects(reply) {
            let value: Value =
                serde_json::from_str(object).map_err(|_| AiError::MalformedMemory {
                    reply: reply.to_string(),
                })?;
            absorb(&value, &mut facts, &mut summary);
        }

        let Some(summary) = summary else {
            return Err(AiError::MalformedMemory {
                reply: reply.to_string(),
            });
        };

        Ok(Self { facts, summary })
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

    let reply = client
        .request(model, &messages, Sampling::EXTRACTION)
        .await?;

    MemoryUpdate::from_reply(&reply)
}

/// Fold one decoded JSON object into the accumulating record.
fn absorb(value: &Value, facts: &mut Vec<String>, summary: &mut Option<String>) {
    if let Some(list) = value.get("facts").and_then(Value::as_array) {
        facts.extend(list.iter().filter_map(Value::as_str).map(str::to_string));
    }

    if let Some(text) = value.get("summary").and_then(Value::as_str)
        && !text.trim().is_empty()
    {
        *summary = Some(text.to_string());
    }
}

/// Every top-level `{...}` object in `text`, ignoring braces inside strings.
/// TODO: Make usage of a lib for this algorithm
fn json_objects(text: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in text.char_indices() {
        if in_string {
            match character {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => in_string = false,
                _ => escaped = false,
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => {
                start.get_or_insert(index);
                depth += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                if depth == 0
                    && let Some(begin) = start.take()
                {
                    objects.push(&text[begin..=index]);
                }
            }
            _ => {}
        }
    }

    objects
}
