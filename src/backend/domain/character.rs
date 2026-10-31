use std::{collections::HashMap, fmt::Display, vec};

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

impl From<String> for CharacterStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "away" => CharacterStatus::Away,
            "present" => CharacterStatus::Present,
            "unavailable" => CharacterStatus::Unavailable,
            _ => CharacterStatus::Present,
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
    pub adventure_id: Uuid,
    pub name: String,
    pub description: String,
    pub status: CharacterStatus,
    pub backstory: String,
    pub portrait_path: Option<String>,
    pub is_player: bool,
    pub traits: Vec<String>,    // Speaks only in tidal registers, ...
    pub tags: Vec<String>,      // Protagonist, Brave ...
    pub relations: Vec<String>, // Son of Dragoon
}

impl Character {
    pub const DEFAULT_PLAYER_NAME: &'static str = "You";

    pub fn new(
        id: Uuid,
        name: String,
        adventure_id: Uuid,
        description: impl Into<String>,
        backstory: impl Into<String>,
        status: CharacterStatus,
        is_player: bool,
    ) -> Self {
        Self {
            id,
            adventure_id,
            name,
            description: description.into(),
            backstory: backstory.into(),
            status,
            portrait_path: None,
            is_player,
            traits: vec![],
            tags: vec![],
            relations: vec![],
        }
    }

    pub fn with_portrait(mut self, portrait_path: impl Into<String>) -> Self {
        self.portrait_path = Some(portrait_path.into());
        self
    }

    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }

    pub fn add_trait(&mut self, trait_: String) {
        self.traits.push(trait_);
    }

    pub fn remove_tag(&mut self, tag: String) {
        self.tags.retain(|t| t != &tag);
    }

    pub fn add_relation(&mut self, relation: String) {
        self.relations.push(relation);
    }

    pub fn render(&self) -> String {
        let mut parts = vec![format!("{}.", self.name)];
        if !self.description.trim().is_empty() {
            parts.push(self.description.clone());
        }
        if !self.backstory.trim().is_empty() {
            parts.push(format!("Backstory: {}", self.backstory));
        }
        if !self.traits.is_empty() {
            parts.push(format!("Traits: {}.", self.traits.join(", ")));
        }

        parts.join(" ")
    }
}
