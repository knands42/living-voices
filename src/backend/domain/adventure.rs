use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::backend::domain::{
    character::Character, lore::Lore, message::Message, narrator::Narrator,
};

// TODO: confirm if is a good pattern
pub struct AdventureSummary {
    pub id: Uuid,
    pub title: String,
    pub headline: String,
    pub tone: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AdventureSummary {
    pub fn new(
        id: Uuid,
        title: impl Into<String>,
        headline: impl Into<String>,
        tone: impl Into<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            headline: headline.into(),
            tone: tone.into(),
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
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            title: title.into(),
            headline: headline.into(),
            narrator,
            lore,
            characters: Vec::new(),
            tone: tone.into(),
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

    pub fn find_character(&self, name: &str) -> Option<&Character> {
        self.characters.iter().find(|c| c.name == name)
    }

    pub fn active_characters(&self, present_names: &[String]) -> Vec<&Character> {
        self.characters
            .iter()
            .filter(|c| present_names.iter().any(|name| name == &c.name))
            .collect()
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

        Message::new(
            self.id,
            character_id,
            narrator_id,
            speaker_name,
            text,
            false,
            vec![], // TODO: Get targeted characters
        )
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
        if let Some(end) = s.find(']') {
            if s.starts_with('[') {
                s = s[end + 1..].trim_start();
            }
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
