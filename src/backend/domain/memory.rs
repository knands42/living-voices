use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A compressed span of an adventure's message log, covering the `message.seq`
/// values `from_seq..=to_seq` inclusive.
#[derive(Clone, Debug)]
pub struct Summary {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub text: String,
    pub from_seq: i64,
    pub to_seq: i64,
    pub created_at: DateTime<Utc>,
}

impl Summary {
    pub fn new(adventure_id: Uuid, text: String, from_seq: i64, to_seq: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            text,
            from_seq,
            to_seq,
            created_at: Utc::now(),
        }
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
}
