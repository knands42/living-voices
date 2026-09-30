use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::backend::domain::{character::Character, lore::Lore};

#[derive(Clone, Debug)]
pub struct Adventure {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub lore: Lore,
    pub characters: Vec<Character>,
    pub metadata: AdventureMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Adventure {
    pub fn new(title: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description,
            lore: Lore::new(),
            characters: Vec::new(),
            metadata: AdventureMetadata::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct AdventureMetadata {
    pub genre: Option<String>,
    pub tone: Option<String>,
    pub tags: Vec<String>,
}

impl AdventureMetadata {
    pub fn add_tags(&mut self, tags: Vec<String>) {
        self.tags.extend(tags);
    }
}
