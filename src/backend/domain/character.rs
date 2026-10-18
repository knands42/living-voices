use std::{collections::HashMap, fmt::Display};

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterStatus {
    Present,
    Away,
    Unavailable,
}

impl CharacterStatus {
    pub const ALL: [CharacterStatus; 3] = [
        CharacterStatus::Present,
        CharacterStatus::Away,
        CharacterStatus::Unavailable,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            CharacterStatus::Away => "away",
            CharacterStatus::Present => "present",
            CharacterStatus::Unavailable => "unavailable",
        }
    }
}

impl Display for CharacterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for CharacterStatus {
    fn default() -> Self {
        CharacterStatus::Present
    }
}

#[derive(Clone, Debug)]
pub struct Character {
    pub id: Uuid,
    pub adventure_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub status: CharacterStatus,
    pub backstory: String,
    pub traits: Vec<String>, // Speaks only in tidal registers, ...
    pub tags: Vec<String>,   // Protagonist, Brave ...
    pub relations: HashMap<Uuid, String>, // Mother, Father ...
}

impl Character {
    pub fn new(name: String, adventure_id: Option<Uuid>, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            name,
            description,
            backstory: String::new(),
            status: CharacterStatus::default(),
            traits: Vec::new(),
            tags: Vec::new(),
            relations: HashMap::new(),
        }
    }

    pub fn with_backstory(mut self, backstory: String) -> Self {
        self.backstory = backstory;
        self
    }

    pub fn with_status(mut self, status: CharacterStatus) -> Self {
        self.status = status;
        self
    }

    pub fn add_tags(&mut self, tag: String) {
        self.tags.push(tag);
    }

    pub fn add_trait(&mut self, trait_: String) {
        self.traits.push(trait_);
    }

    pub fn remove_tag(&mut self, tag: String) {
        self.tags.retain(|t| t != &tag);
    }

    pub fn add_relation(&mut self, entity_id: Uuid, relation: String) {
        self.relations.insert(entity_id, relation);
    }

    pub fn render(&self, adventure_tone: Option<&str>) -> String {
        let mut parts = vec![format!("You are {}.", self.name)];
        if !self.description.trim().is_empty() {
            parts.push(self.description.clone());
        }
        if !self.traits.is_empty() {
            parts.push(format!("Traits: {}.", self.traits.join(", ")));
        }
        if !self.backstory.trim().is_empty() {
            parts.push(format!("Backstory: {}", self.backstory));
        }
        if let Some(tone) = adventure_tone {
            if !tone.trim().is_empty() {
                parts.push(format!("Overall tone: {tone}."));
            }
        }

        parts.join(" ")
    }
}
