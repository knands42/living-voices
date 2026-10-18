use chrono::{DateTime, Utc};
use std::fmt::Display;
use uuid::Uuid;

/// How a message is spoken. Mirrors the vocabulary the DB enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MessageKind {
    Narration,
    #[default]
    Say,
    Do,
    Think,
    Whisper,
}

impl MessageKind {
    pub const ALL: [MessageKind; 5] = [
        MessageKind::Narration,
        MessageKind::Say,
        MessageKind::Do,
        MessageKind::Think,
        MessageKind::Whisper,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            MessageKind::Narration => "narration",
            MessageKind::Say => "say",
            MessageKind::Do => "do",
            MessageKind::Think => "think",
            MessageKind::Whisper => "whisper",
        }
    }

    pub fn from_str(s: &str) -> Option<MessageKind> {
        match s {
            "narration" => Some(MessageKind::Narration),
            "say" => Some(MessageKind::Say),
            "do" => Some(MessageKind::Do),
            "think" => Some(MessageKind::Think),
            "whisper" => Some(MessageKind::Whisper),
            _ => None,
        }
    }
}

impl Display for MessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One action a message carries: a kind, its text, and an optional target
/// (`say`/`whisper`/`do` — an action can be done to someone too).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessagePart {
    pub kind: MessageKind,
    pub text: String,
    /// when using the kind `say`/`whisper`/`do`.
    pub target_name: Option<String>,
}

impl MessagePart {
    pub fn new(kind: MessageKind, text: String) -> Self {
        Self {
            kind,
            target_name: None,
            text,
        }
    }

    pub fn with_target(mut self, target_name: String) -> Self {
        self.target_name = Some(target_name);
        self
    }
}

/// The actions a single message carries, e.g. whispering while doing something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageParts {
    primary: MessagePart,
    secondary: Option<MessagePart>,
}

impl MessageParts {
    pub fn one(part: MessagePart) -> Self {
        Self {
            primary: part,
            secondary: None,
        }
    }

    pub fn two(primary: MessagePart, secondary: MessagePart) -> Self {
        Self {
            primary,
            secondary: Some(secondary),
        }
    }

    pub fn primary(&self) -> &MessagePart {
        &self.primary
    }

    pub fn secondary(&self) -> Option<&MessagePart> {
        self.secondary.as_ref()
    }

    pub fn iter(&self) -> impl Iterator<Item = &MessagePart> {
        std::iter::once(&self.primary).chain(self.secondary.as_ref())
    }
}

/// One entry in an adventure's conversation log.
/// Messages are append-only.
#[derive(Clone, Debug)]
pub struct Message {
    /// it is `None` until persisted and always `Some` when read back.
    pub seq: Option<u64>,
    pub adventure_id: Uuid,
    pub character_id: Option<Uuid>,
    pub narrator_id: Option<Uuid>,
    pub speaker_name: String,
    pub parts: MessageParts,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub const NARRATOR_SPEAKER_NAME: &'static str = "NARRATOR";
    pub fn new(
        adventure_id: Uuid,
        character_id: Option<Uuid>,
        narrator_id: Option<Uuid>,
        speaker_name: String,
        parts: MessageParts,
    ) -> Self {
        Self {
            seq: None,
            adventure_id,
            character_id,
            narrator_id,
            speaker_name,
            parts,
            created_at: Utc::now(),
        }
    }
}
