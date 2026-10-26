use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::backend::domain::{
    character::Character, lore::Lore, message::Message, narrator::Narrator,
};

#[derive(Clone, Debug)]
pub struct Adventure {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub narrator: Narrator,
    pub lore: Lore,
    pub characters: Vec<Character>,
    pub genre: Option<String>,
    pub tone: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Adventure {
    pub fn new(title: String, description: String, narrator_name: String, tone: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description,
            narrator: Narrator::new(String::from(narrator_name)),
            lore: Lore::new(),
            characters: Vec::new(),
            genre: None,
            tone: String::from(tone),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_tags(&mut self, tags: Vec<String>) {
        self.tags.extend(tags);
    }

    pub fn find_character(&self, name: &str) -> Option<&Character> {
        self.characters.iter().find(|c| c.name == name)
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
}
