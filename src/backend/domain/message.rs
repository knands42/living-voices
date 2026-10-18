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
    Ooc, // out of character
}

impl MessageKind {
    pub const ALL: [MessageKind; 5] = [
        MessageKind::Narration,
        MessageKind::Say,
        MessageKind::Do,
        MessageKind::Think,
        MessageKind::Ooc,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            MessageKind::Narration => "narration",
            MessageKind::Say => "say",
            MessageKind::Do => "do",
            MessageKind::Think => "think",
            MessageKind::Ooc => "ooc",
        }
    }

    pub fn from_str(s: &str) -> Option<MessageKind> {
        match s {
            "narration" => Some(MessageKind::Narration),
            "say" => Some(MessageKind::Say),
            "do" => Some(MessageKind::Do),
            "think" => Some(MessageKind::Think),
            "ooc" => Some(MessageKind::Ooc),
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
    /// nuance applied to the kind: "in fear", "half asleep", "sarcastic"
    pub nuance: Option<String>,
}

impl MessagePart {
    pub fn new(kind: MessageKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            target_name: None,
            text: text.into(),
            nuance: None,
        }
    }

    pub fn with_target(mut self, target_name: impl Into<String>) -> Self {
        self.target_name = Some(target_name.into());
        self
    }

    pub fn render(&self, speaker: &str) -> String {
        let nuance = self.nuance();
        let mode = |label: &str| match nuance.as_deref() {
            Some(n) => format!("({label} {n})"),
            None => format!("({label})"),
        };

        let text = &self.text;
        match (self.kind, self.target_name.as_deref()) {
            (MessageKind::Say, Some(target)) => {
                format!("[{speaker} {} -> {target}]: {text}", mode("say"))
            }
            (MessageKind::Say, None) => format!("[{speaker} (say)]: {text}"),
            (MessageKind::Think, _) => {
                format!("[{speaker} {}]: {text}", mode("think"))
            }
            (MessageKind::Do, None) => format!("[{speaker} {}]: {text}", mode("do")),
            (MessageKind::Do, Some(target)) => {
                format!("[{speaker} {} -> {target}]: {text}", mode("do"))
            }
            (MessageKind::Ooc, _) => format!("[{speaker} (out of character)]: {text}"),
            (MessageKind::Narration, _) => format!("[narrator]: {text}"),
        }
    }

    fn nuance(&self) -> Option<String> {
        let n = clean(self.nuance.clone()?);
        if n.is_empty() {
            None
        } else {
            Some(n.chars().take(48).collect::<String>())
        }
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

    pub fn render(&self, speaker: &str) -> String {
        self.iter()
            .map(|part| part.render(speaker))
            .collect::<Vec<_>>()
            .join("\n")
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

    pub fn render(messages: &[Message]) -> Option<String> {
        if messages.is_empty() {
            return None;
        }

        let mut block = String::new();
        for message in messages {
            block.push_str(&message.parts.render(&message.speaker_name));
            block.push('\n');
        }

        Some(block)
    }
}

fn clean(raw: String) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '(' | ')' | ':' | '*' | '[' | ']' | '<' | '>' => out.push(' '),
            c if c.is_control() || c == '\n' => out.push(' '),
            c => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
