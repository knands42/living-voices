use chrono::{DateTime, Utc};
use uuid::Uuid;

/// One closed scene: the retrieval unit, covering `message.seq` `from_seq..=to_seq`.
#[derive(Clone, Debug)]
pub struct Episode {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub act_id: Option<Uuid>,
    pub scene: String,
    pub summary: String,
    pub importance: i64,
    pub entities: Vec<String>,
    pub embedding: Option<Vec<u8>>,
    pub from_seq: u64,
    pub to_seq: u64,
    pub created_at: DateTime<Utc>,
}

impl Episode {
    pub fn new(
        adventure_id: Uuid,
        scene: impl Into<String>,
        summary: impl Into<String>,
        importance: i64,
        entities: Vec<String>,
        from_seq: u64,
        to_seq: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            act_id: None,
            scene: scene.into(),
            summary: summary.into(),
            importance,
            entities,
            embedding: None,
            from_seq,
            to_seq,
            created_at: Utc::now(),
        }
    }

    pub fn in_act(mut self, act_id: Uuid) -> Self {
        self.act_id = Some(act_id);
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<u8>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn render(episodes: &[Episode]) -> Option<String> {
        if episodes.is_empty() {
            return None;
        }

        let mut block = String::from("Story so far: ");
        for episode in episodes {
            block.push_str(&format!("\n- [{}] {}", episode.scene, episode.summary));
        }
        Some(block)
    }
}
