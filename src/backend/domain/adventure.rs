use std::fmt::Display;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::backend::domain::{
    character::Character, lore::Lore, message::Message, narrator::Narrator,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AdventureStatus {
    #[default]
    Draft,
    Private,
    Public,
}

impl AdventureStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            AdventureStatus::Draft => "draft",
            AdventureStatus::Private => "private",
            AdventureStatus::Public => "public",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "private" => AdventureStatus::Private,
            "public" => AdventureStatus::Public,
            _ => AdventureStatus::Draft,
        }
    }
}

impl Display for AdventureStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for AdventureStatus {
    fn from(value: String) -> Self {
        AdventureStatus::from_str(value.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct AdventureSummary {
    pub id: Uuid,
    pub title: String,
    pub headline: String,
    pub tone: String,
    pub status: AdventureStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AdventureSummary {
    pub fn new(
        id: Uuid,
        title: impl Into<String>,
        headline: impl Into<String>,
        tone: impl Into<String>,
        status: AdventureStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            headline: headline.into(),
            tone: tone.into(),
            status,
            created_at,
            updated_at,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Adventure {
    pub id: Uuid,
    pub title: String,
    pub headline: String,
    pub narrator: Narrator,
    pub lore: Lore,
    pub tone: String,
    pub status: AdventureStatus,
    pub characters: Vec<Character>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Adventure {
    pub fn new(
        id: Uuid,
        title: impl Into<String>,
        headline: impl Into<String>,
        narrator: Narrator,
        tone: impl Into<String>,
        lore: Lore,
        status: AdventureStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            headline: headline.into(),
            narrator,
            lore,
            characters: Vec::new(),
            tone: tone.into(),
            status,
            tags: Vec::new(),
            created_at,
            updated_at,
        }
    }

    pub fn add_character(&mut self, character: Character) {
        self.characters.push(character);
    }

    pub fn add_tags(&mut self, tags: Vec<String>) {
        self.tags.extend(tags);
    }

    pub fn find_character(&self, character_id: Uuid) -> Option<&Character> {
        self.characters.iter().find(|c| c.id == character_id)
    }

    pub fn active_characters(&self, present_names: &[String]) -> Vec<&Character> {
        self.characters
            .iter()
            .filter(|c| present_names.iter().any(|name| name == &c.name))
            .collect()
    }

    pub fn player_name(&self) -> Option<&str> {
        self.characters
            .iter()
            .find(|c| c.is_player)
            .map(|c| c.name.as_str())
    }

    pub fn message_from_reply(&self, replier: &str, text: &str) -> Message {
        let character = if Adventure::is_narrator_reply(replier) {
            None
        } else {
            self.resolve_replier(replier)
        };

        let (character_id, narrator_id, speaker_name) = match character {
            Some(character) => (Some(character.id), None, character.name.clone()),
            None => (
                None,
                Some(self.narrator.id),
                Message::NARRATOR_SPEAKER_NAME.to_string(),
            ),
        };

        let body = Adventure::strip_speaker_prefix(text, &speaker_name);

        Message::new(
            self.id,
            character_id,
            narrator_id,
            speaker_name,
            body,
            false,
        )
    }

    /// The model sometimes echoes the speaker into the reply body as `"Name: ..."` -> Remove leftovers
    fn strip_speaker_prefix<'a>(text: &'a str, speaker_name: &str) -> &'a str {
        let trimmed = text.trim_start();
        let Some((head, rest)) = trimmed.split_once(':') else {
            return text;
        };

        if head.trim().eq_ignore_ascii_case(speaker_name.trim()) {
            rest.trim_start()
        } else {
            text
        }
    }

    fn resolve_replier(&self, replier: &str) -> Option<&Character> {
        let cleaned = Adventure::clean_replier(replier);
        if cleaned.is_empty() {
            return None;
        }
        self.characters
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(&cleaned))
    }

    fn clean_replier(replier: &str) -> String {
        let mut s = replier.trim();

        // Drop a leading header tag like `[CHARACTER]` or `[NARRATOR]`.
        if let Some(end) = s.find(']')
            && s.starts_with('[')
        {
            s = s[end + 1..].trim_start();
        }

        // Drop a trailing `| target: ...` or any trailing `|`/`:` qualifier.
        if let Some(idx) = s.find('|') {
            s = s[..idx].trim_end();
        }
        if let Some(idx) = s.find(':') {
            s = s[..idx].trim_end();
        }

        s.to_string()
    }

    fn is_narrator_reply(replier: &str) -> bool {
        let cleaned = Adventure::clean_replier(replier);
        cleaned.is_empty() || cleaned.eq_ignore_ascii_case(Message::NARRATOR_SPEAKER_NAME)
    }
}
