use std::fmt::Display;

use chrono::{DateTime, Utc};
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
}

impl Display for MessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One entry in an adventure's conversation log.
/// Messages are append-only
#[derive(Clone, Debug)]
pub struct Message {
    /// it is `None` until persisted and always `Some` when read back.
    pub seq: Option<i64>,
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub character_id: Uuid,
    pub speaker_name: String,
    pub kind: MessageKind,
    pub text: String,
    /// when using the option `say`/`whisper`.
    pub target_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub const NARRATOR_SPEAKER_NAME: &'static str = "NARRATOR";
    pub fn new(adventure_id: Uuid, character_id: Uuid, speaker_name: String, kind: MessageKind, text: String) -> Self {
        Self {
            seq: None,
            id: Uuid::new_v4(),
            adventure_id,
            character_id,
            speaker_name,
            kind,
            text,
            target_name: None,
            created_at: Utc::now(),
        }
    }
    pub fn with_target(mut self, target_name: String) -> Self {
        self.target_name = Some(target_name);
        self
    }
}
