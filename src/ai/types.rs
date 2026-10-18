use crate::backend::domain::message::Message;
use serde::{Deserialize, Serialize};

/// Who authored a message in a model conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// One turn of a conversation sent to a chat-completion model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

/// Render a stored message as the conversation line handed to a model:
/// `[<speaker> <kind>] to [<target>]: <text>` per part, parts joined with ` + `,
/// e.g. `[Alice whisper] to [Bob]: come here + [Alice do]: raise the drawing`.
impl From<&Message> for ChatMessage {
    fn from(message: &Message) -> Self {
        let rendered = message
            .parts
            .iter()
            .map(|part| {
                let mut segment = format!("[{} {}]", message.speaker_name, part.kind);
                if let Some(target) = &part.target_name {
                    segment.push_str(&format!(" to [{target}]"));
                }
                segment.push_str(&format!(": {}", part.text));
                segment
            })
            .collect::<Vec<_>>()
            .join(" + ");
        ChatMessage::user(rendered)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sampling {
    /// Low values stay faithful to the prompt
    pub temperature: f32,
}

impl Sampling {
    /// Character dialogue: expressive but coherent.
    pub const CHAT: Self = Self { temperature: 0.5 };

    /// Memory and summary extraction: faithful, no invention.
    pub const EXTRACTION: Self = Self { temperature: 0.1 };
}
