use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A compressed span of an adventure's log, covering `message.seq` values `from_seq..=to_seq`.
#[derive(Clone, Debug)]
pub struct Summary {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub text: String,
    pub from_seq: u64,
    pub to_seq: u64,
    pub created_at: DateTime<Utc>,
}

impl Summary {
    pub fn new(adventure_id: Uuid, text: String, from_seq: u64, to_seq: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            text,
            from_seq,
            to_seq,
            created_at: Utc::now(),
        }
    }

    pub fn render(summaries: &[Summary]) -> Option<String> {
        if summaries.is_empty() {
            return None;
        }

        let mut block = String::new();
        block.push_str("Story so far: ");

        for summary in summaries {
            block.push_str(&format!("\n- {}", summary.text));
        }

        Some(block)
    }
}

/// Facts that lives alongside a [`Summary`].
#[derive(Clone, Debug)]
pub struct Fact {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub source_summary_id: Uuid,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

impl Fact {
    pub fn new(adventure_id: Uuid, source_summary_id: Uuid, text: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            source_summary_id,
            text,
            created_at: Utc::now(),
        }
    }

    pub fn render(facts: &[Fact]) -> Option<String> {
        if facts.is_empty() {
            return None;
        }

        let mut block = String::new();
        block.push_str("\nKnown facts:");

        for fact in facts {
            block.push_str(&format!("\n- {}", fact.text));
        }

        Some(block)
    }
}
