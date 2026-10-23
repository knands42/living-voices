use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::{
    client::{JsonSchema, OpenAiCompatibleClient},
    error::AiError,
    types::ChatMessage,
};
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::application::port::memory_port::{
    DistillError, DistilledFact, DistilledMemory, DistilledState, FactPermanence, MemoryDistiller,
};
use crate::backend::domain::message::Message;
use crate::config::TextModel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permanence {
    Permanent,
    Plot,
}

impl From<Permanence> for FactPermanence {
    fn from(value: Permanence) -> Self {
        match value {
            Permanence::Permanent => FactPermanence::Permanent,
            Permanence::Plot => FactPermanence::Plot,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub subject: String,
    pub fact: String,
    pub permanence: Permanence,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateSnapshot {
    #[serde(default)]
    pub day: Option<String>,
    #[serde(default)]
    pub place: Option<String>,
    #[serde(default)]
    pub characters_present: Vec<String>,
    #[serde(default)]
    pub open_threads: Vec<String>,
    #[serde(default)]
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryUpdate {
    pub facts: Vec<Fact>,
    pub state: StateSnapshot,
    #[serde(skip)]
    config: TextModel,
}

impl MemoryUpdate {
    pub fn new(config: TextModel) -> Self {
        Self {
            facts: vec![],
            state: StateSnapshot::default(),
            config,
        }
    }

    pub fn from_reply(reply: &str) -> Result<Self, AiError> {
        serde_json::from_str(reply).map_err(|_| AiError::MalformedMemory {
            reply: reply.to_string(),
        })
    }

    pub async fn distill(
        client: &OpenAiCompatibleClient,
        model: &str,
        temperature: f32,
        prompt: &[ChatMessage],
    ) -> Result<MemoryUpdate, AiError> {
        let mut messages = Vec::with_capacity(prompt.len() + 1);
        messages.extend_from_slice(prompt);

        let format = crate::ai::client::ResponseFormat::JsonSchema {
            json_schema: &schema(),
        };

        let reply = client
            .send(model, &messages, temperature, Some(format))
            .await?;

        MemoryUpdate::from_reply(&reply)
    }
}

impl MemoryDistiller for MemoryUpdate {
    async fn distill(&self, context: &[PromptTurn]) -> Result<DistilledMemory, DistillError> {
        let prompt = context
            .iter()
            .cloned()
            .map(ChatMessage::from)
            .collect::<Vec<ChatMessage>>();

        let client = OpenAiCompatibleClient::with_timeout(
            &self.config.base_url,
            std::time::Duration::from_secs(self.config.request_timeout_secs),
        )
        .map_err(|e| DistillError::Provider(e.to_string()))?;
        let update = MemoryUpdate::distill(
            &client,
            &self.config.model,
            self.config.temperature,
            &prompt,
        )
        .await
        .map_err(|e| DistillError::Provider(e.to_string()))?;

        Ok(DistilledMemory::from(update))
    }
}

impl From<MemoryUpdate> for DistilledMemory {
    fn from(memory_update: MemoryUpdate) -> Self {
        let facts = memory_update
            .facts
            .into_iter()
            .map(|fact| DistilledFact {
                subject: fact.subject,
                fact: fact.fact,
                permanence: fact.permanence.into(),
            })
            .collect();

        let state = DistilledState {
            day: memory_update.state.day,
            place: memory_update.state.place,
            characters_present: memory_update.state.characters_present,
            open_threads: memory_update.state.open_threads,
            conditions: memory_update.state.conditions,
        };

        DistilledMemory { facts, state }
    }
}

fn schema() -> JsonSchema {
    JsonSchema {
        name: "memory_update".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "facts": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "subject": { "type": "string" },
                            "fact": { "type": "string" },
                            "permanence": {
                                "type": "string",
                                "enum": ["permanent", "plot"]
                            }
                        },
                        "required": ["subject", "fact", "permanence"],
                        "additionalProperties": false
                    }
                },
                "state": {
                    "type": "object",
                    "properties": {
                        "day": { "type": ["string", "null"] },
                        "place": { "type": ["string", "null"] },
                        "characters_present": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "open_threads": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "conditions": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": [
                        "day",
                        "place",
                        "characters_present",
                        "open_threads",
                        "conditions"
                    ],
                    "additionalProperties": false
                }
            },
            "required": ["facts", "state"],
            "additionalProperties": false
        }),
    }
}
