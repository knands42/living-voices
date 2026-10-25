use crate::backend::domain::character::Character;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Append-only entry in an adventure's conversation log, holding ready-formatted text.
#[derive(Clone, Debug)]
pub struct Message {
    pub seq: Option<u16>,
    pub adventure_id: Uuid,
    pub character_id: Option<Uuid>,
    pub narrator_id: Option<Uuid>,
    pub speaker_name: String,
    pub text: String,
    pub targeted_characters: Option<Vec<Character>>,
    pub ooc: bool,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub const NARRATOR_SPEAKER_NAME: &'static str = "NARRATOR";

    pub fn new(
        adventure_id: Uuid,
        character_id: Option<Uuid>,
        narrator_id: Option<Uuid>,
        speaker_name: String,
        text: impl Into<String>,
        ooc: bool,
        targeted_characters: Option<Vec<Character>>,
    ) -> Self {
        Self {
            seq: None,
            adventure_id,
            character_id,
            narrator_id,
            speaker_name,
            text: text.into(),
            targeted_characters,
            ooc,
            created_at: Utc::now(),
        }
    }
}
