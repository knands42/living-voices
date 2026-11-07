use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::error::AiError;
use crate::ai::integrations::text::open_api_client::{JsonSchema, OpenAiCompatibleClient};
use crate::ai::types::ChatMessage;
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::application::port::memory_port::{
    ActCloserPort, ClosedAct, ClosedActError, ClosedActStatus, DistillError, DistilledFact,
    DistilledMemory, DistilledState, FactPermanence, MemoryDistillerPort,
};
use crate::backend::domain::memory::CharacterCondition;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterConditionSnapshot {
    pub character: String,
    pub note: String,
}

impl From<CharacterConditionSnapshot> for CharacterCondition {
    fn from(value: CharacterConditionSnapshot) -> Self {
        CharacterCondition::new(value.character, value.note)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateSnapshot {
    #[serde(default = "unknown", deserialize_with = "or_unknown")]
    pub day: String,
    #[serde(default = "unknown", deserialize_with = "or_unknown")]
    pub place: String,
    #[serde(default, deserialize_with = "or_empty")]
    pub characters_present: Vec<String>,
    #[serde(default, deserialize_with = "or_empty")]
    pub open_threads: Vec<String>,
    #[serde(default)]
    pub character_conditions: Vec<CharacterConditionSnapshot>,
}

fn unknown() -> String {
    String::from("unknown")
}

fn or_unknown<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_else(unknown))
}

fn or_empty<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    Ok(Option::<Vec<String>>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryHandler {
    pub facts: Vec<Fact>,
    pub state: StateSnapshot,
    #[serde(default)]
    pub running_digest: String,
    #[serde(skip)]
    config: TextModel,
}

impl MemoryHandler {
    pub fn new(config: TextModel) -> Self {
        Self {
            facts: vec![],
            state: StateSnapshot::default(),
            running_digest: String::new(),
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
        keep_alive: Option<&str>,
    ) -> Result<MemoryHandler, AiError> {
        let mut messages = Vec::with_capacity(prompt.len() + 1);
        messages.extend_from_slice(prompt);

        let format = crate::ai::integrations::text::open_api_client::ResponseFormat::JsonSchema {
            json_schema: &schema(),
        };

        let reply = client
            .send(model, &messages, temperature, Some(format), keep_alive)
            .await?;

        MemoryHandler::from_reply(&reply)
    }
}

impl MemoryDistillerPort for MemoryHandler {
    async fn distill(&self, context: &[PromptTurn]) -> Result<DistilledMemory, DistillError> {
        let prompt = context
            .iter()
            .cloned()
            .map(ChatMessage::from)
            .collect::<Vec<ChatMessage>>();

        let client = OpenAiCompatibleClient::with_timeout(
            &self.config.base_url,
            std::time::Duration::from_secs(self.config.request_timeout_secs as u64),
        )
        .map_err(|e| DistillError::Provider(e.to_string()))?;
        let update = MemoryHandler::distill(
            &client,
            &self.config.model,
            self.config.temperature,
            &prompt,
            super::keep_alive(&self.config),
        )
        .await
        .map_err(|e| DistillError::Provider(e.to_string()))?;

        Ok(DistilledMemory::from(update))
    }
}

impl From<MemoryHandler> for DistilledMemory {
    fn from(memory_handler: MemoryHandler) -> Self {
        let facts = memory_handler
            .facts
            .into_iter()
            .map(|fact| DistilledFact {
                subject: fact.subject,
                fact: fact.fact,
                permanence: fact.permanence.into(),
            })
            .collect();

        let state = DistilledState {
            day: memory_handler.state.day,
            place: memory_handler.state.place,
            characters_present: memory_handler.state.characters_present,
            open_threads: memory_handler.state.open_threads,
            character_conditions: memory_handler
                .state
                .character_conditions
                .into_iter()
                .map(CharacterCondition::from)
                .collect(),
        };

        DistilledMemory {
            facts,
            state,
            running_digest: memory_handler.running_digest,
        }
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
                        "day": {
                            "type": ["string"],
                            "default": ""
                        },
                        "place": {
                            "type": ["string"],
                            "default": ""
                        },
                        "characters_present": {
                            "type": "array",
                            "items": { "type": "string" },
                            "default": []
                        },
                        "open_threads": {
                            "type": "array",
                            "items": { "type": "string" },
                            "default": []
                        },
                        "character_conditions": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "character": { "type": "string" },
                                    "note": { "type": "string" }
                                },
                                "required": ["character", "note"],
                                "additionalProperties": false
                            },
                            "default": []
                        }
                    },
                    "required": [
                        "day",
                        "place",
                        "characters_present",
                        "open_threads",
                        "character_conditions"
                    ],
                    "additionalProperties": false
                },
                "running_digest": { "type": "string" }
            },
            "required": ["facts", "state", "running_digest"],
            "additionalProperties": false
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClosedStatus {
    Resolved,
    Failed,
    Bypassed,
}

impl From<ClosedStatus> for ClosedActStatus {
    fn from(value: ClosedStatus) -> Self {
        match value {
            ClosedStatus::Resolved => ClosedActStatus::Resolved,
            ClosedStatus::Failed => ClosedActStatus::Failed,
            ClosedStatus::Bypassed => ClosedActStatus::Bypassed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActCloseReply {
    pub index_line: String,
    pub digest: String,
    pub status: ClosedStatus,
}

impl ActCloseReply {
    fn from_reply(reply: &str) -> Result<Self, AiError> {
        serde_json::from_str(reply).map_err(|_| AiError::MalformedMemory {
            reply: reply.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActClose {
    #[serde(skip)]
    config: TextModel,
}

impl ActClose {
    pub fn new(config: TextModel) -> Self {
        Self { config }
    }
}

impl ActCloserPort for ActClose {
    async fn close_act(&self, context: &[PromptTurn]) -> Result<ClosedAct, ClosedActError> {
        let prompt = context
            .iter()
            .cloned()
            .map(ChatMessage::from)
            .collect::<Vec<ChatMessage>>();

        let client = OpenAiCompatibleClient::with_timeout(
            &self.config.base_url,
            std::time::Duration::from_secs(self.config.request_timeout_secs as u64),
        )
        .map_err(|e| ClosedActError::Provider(e.to_string()))?;

        let format = crate::ai::integrations::text::open_api_client::ResponseFormat::JsonSchema {
            json_schema: &act_close_schema(),
        };

        let reply = client
            .send(
                &self.config.model,
                &prompt,
                self.config.temperature,
                Some(format),
                super::keep_alive(&self.config),
            )
            .await
            .map_err(|e| ClosedActError::Provider(e.to_string()))?;

        let parsed = ActCloseReply::from_reply(&reply)
            .map_err(|e| ClosedActError::Provider(e.to_string()))?;

        Ok(ClosedAct {
            index_line: parsed.index_line,
            digest: parsed.digest,
            status: parsed.status.into(),
        })
    }
}

fn act_close_schema() -> JsonSchema {
    JsonSchema {
        name: "act_close".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "index_line": { "type": "string" },
                "digest": { "type": "string" },
                "status": {
                    "type": "string",
                    "enum": ["resolved", "failed", "bypassed"]
                }
            },
            "required": ["index_line", "digest", "status"],
            "additionalProperties": false
        }),
    }
}
