use chrono::{DateTime, Utc};
use slint::PlatformError;
use uuid::Uuid;

/// The single live snapshot of an adventure: exactly one row per adventure.
#[derive(Clone, Debug)]
pub struct State {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub day: String,
    pub place: String,
    pub characters_present: Vec<String>,
    pub open_threads: Vec<String>,
    pub conditions: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

impl State {
    pub fn new(adventure_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            day: String::from("unknown"),
            place: String::from("unknown"),
            characters_present: Vec::new(),
            open_threads: Vec::new(),
            conditions: Vec::new(),
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

    pub fn with_conditions(mut self, conditions: Vec<String>) -> Self {
        self.conditions = conditions;
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
        if !self.conditions.is_empty() {
            lines.push(format!("Wounds: {}", self.conditions.join(", ")));
        }

        if lines.is_empty() {
            return None;
        }

        Some(format!("Current state:\n- {}", lines.join("\n- ")))
    }
}
