use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The single live snapshot of an adventure: exactly one row per adventure.
#[derive(Clone, Debug)]
pub struct State {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub day: Option<String>,
    pub place: Option<String>,
    pub present: Vec<String>,
    pub open_threads: Vec<String>,
    pub wounds: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

impl State {
    pub fn new(adventure_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            day: None,
            place: None,
            present: Vec::new(),
            open_threads: Vec::new(),
            wounds: Vec::new(),
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
        if !self.present.is_empty() {
            lines.push(format!("Present: {}", self.present.join(", ")));
        }
        if !self.open_threads.is_empty() {
            lines.push(format!("Open threads: {}", self.open_threads.join(", ")));
        }
        if !self.wounds.is_empty() {
            lines.push(format!("Wounds: {}", self.wounds.join(", ")));
        }

        if lines.is_empty() {
            return None;
        }
        Some(format!("Current state:\n- {}", lines.join("\n- ")))
    }
}
