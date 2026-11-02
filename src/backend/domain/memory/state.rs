use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single live condition about one character, e.g. a wound that changes how they look.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterCondition {
    pub character: String,
    pub note: String,
}

impl CharacterCondition {
    pub fn new(character: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            character: character.into(),
            note: note.into(),
        }
    }

    pub fn for_character<'a>(
        conditions: &'a [CharacterCondition],
        character: &str,
    ) -> Vec<&'a CharacterCondition> {
        conditions
            .iter()
            .filter(|c| c.character.eq_ignore_ascii_case(character))
            .collect()
    }
}

impl std::fmt::Display for CharacterCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.character, self.note)
    }
}

/// The single live snapshot of an adventure: exactly one row per adventure.
#[derive(Clone, Debug)]
pub struct State {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub day: String,
    pub place: String,
    pub characters_present: Vec<String>,
    pub open_threads: Vec<String>,
    pub character_conditions: Vec<CharacterCondition>,
    pub updated_at: DateTime<Utc>,
}

impl State {
    pub fn new(id: Uuid, adventure_id: Uuid) -> Self {
        Self {
            id,
            adventure_id,
            day: String::from("unknown"),
            place: String::from("unknown"),
            characters_present: Vec::new(),
            open_threads: Vec::new(),
            character_conditions: Vec::new(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_day(mut self, day: String) -> Self {
        self.day = day;
        self
    }

    pub fn with_place(mut self, place: String) -> Self {
        self.place = place;
        self
    }

    pub fn with_character_conditions(
        mut self,
        character_conditions: Vec<CharacterCondition>,
    ) -> Self {
        self.character_conditions = character_conditions;
        self
    }

    pub fn with_characters_present(mut self, characters_present: Vec<String>) -> Self {
        self.characters_present = characters_present;
        self
    }

    pub fn with_open_threads(mut self, open_threads: Vec<String>) -> Self {
        self.open_threads = open_threads;
        self
    }

    pub fn render(&self) -> Option<String> {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("Day: {}", self.day));
        lines.push(format!("Place: {}", self.place));

        if !self.characters_present.is_empty() {
            lines.push(format!("Present: {}", self.characters_present.join(", ")));
        }
        if !self.open_threads.is_empty() {
            lines.push(format!("Open threads: {}", self.open_threads.join(", ")));
        }
        if !self.character_conditions.is_empty() {
            let rendered: Vec<String> = self
                .character_conditions
                .iter()
                .map(CharacterCondition::to_string)
                .collect();
            lines.push(format!("Conditions: {}", rendered.join(", ")));
        }

        if lines.is_empty() {
            return None;
        }

        Some(format!("Current state:\n- {}", lines.join("\n- ")))
    }
}
