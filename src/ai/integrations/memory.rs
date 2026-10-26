use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::{
    client::{JsonSchema, OpenAiCompatibleClient},
    error::AiError,
    types::ChatMessage,
};
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::application::port::memory_port::{
    ActCloser, ClosedAct, ClosedActError, ClosedActStatus, DistillError, DistilledFact,
    DistilledMemory, DistilledState, FactPermanence, MemoryDistiller,
};
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
    #[serde(default = "unknown", deserialize_with = "or_unknown")]
    pub day: String,
    #[serde(default = "unknown", deserialize_with = "or_unknown")]
    pub place: String,
    #[serde(default, deserialize_with = "or_empty")]
    pub characters_present: Vec<String>,
    #[serde(default, deserialize_with = "or_empty")]
    pub open_threads: Vec<String>,
    #[serde(default, deserialize_with = "or_empty")]
    pub conditions: Vec<String>,
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
pub struct MemoryUpdate {
    pub facts: Vec<Fact>,
    pub state: StateSnapshot,
    #[serde(default)]
    pub running_digest: String,
    #[serde(skip)]
    config: TextModel,
}

impl MemoryUpdate {
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
    ) -> Result<MemoryUpdate, AiError> {
        let mut messages = Vec::with_capacity(prompt.len() + 1);
        messages.extend_from_slice(prompt);

        let format = crate::ai::client::ResponseFormat::JsonSchema {
            json_schema: &schema(),
        };

        let reply = client
            .send(model, &messages, temperature, Some(format), keep_alive)
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
            std::time::Duration::from_secs(self.config.request_timeout_secs as u64),
        )
        .map_err(|e| DistillError::Provider(e.to_string()))?;
        let update = MemoryUpdate::distill(
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

        DistilledMemory {
            facts,
            state,
            running_digest: memory_update.running_digest,
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
                        "conditions": {
                            "type": "array",
                            "items": { "type": "string" },
                            "default": []
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

impl ActCloser for ActClose {
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

        let format = crate::ai::client::ResponseFormat::JsonSchema {
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
