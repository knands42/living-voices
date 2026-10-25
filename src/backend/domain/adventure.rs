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
            narrator: Narrator::new(String::from(narrator_name), None),
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

    pub fn active_characters(&self, present_names: &[String]) -> Vec<&Character> {
        self.characters
            .iter()
            .filter(|c| present_names.iter().any(|name| name == &c.name))
            .collect()
    }

    pub fn message_from_reply(&self, replier: &str, text: &str) -> Message {
        let (character_id, narrator_id, speaker_name) = match self.find_character(replier) {
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
            vec![],
        )
    }
}
