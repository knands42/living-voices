use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Character {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub backstory: String,
    pub traits: Vec<String>, // Speaks only in tidal registers, ...
    pub tags: Vec<String>,   // Protagonist, Brave ...
    pub relations: HashMap<Uuid, String>, // Mother, Father ...
}

impl Character {
    pub fn new(name: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            backstory: String::new(),
            traits: Vec::new(),
            tags: Vec::new(),
            relations: HashMap::new(),
        }
    }

    pub fn with_backstory(mut self, backstory: String) -> Self {
        self.backstory = backstory;
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
}
