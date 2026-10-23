use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The single live snapshot of an adventure: exactly one row per adventure.
#[derive(Clone, Debug)]
pub struct State {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub day: Option<String>,
    pub place: Option<String>,
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
            day: None,
            place: None,
            characters_present: Vec::new(),
            open_threads: Vec::new(),
            conditions: Vec::new(),
            updated_at: Utc::now(),
        }
    }

    pub fn render(&self) -> Option<String> {
        let mut lines: Vec<String> = Vec::new();
        if let Some(day) = &self.day {
            lines.push(format!("Day: {day}"));
        }
        if let Some(place) = &self.place {
            lines.push(format!("Place: {place}"));
        }
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
