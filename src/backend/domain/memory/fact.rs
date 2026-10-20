use std::fmt::Display;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// How long a fact is expected to hold. Mirrors the vocabulary the DB enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Permanence {
    Permanent,
    #[default]
    Plot,
    Scene,
}

impl Permanence {
    pub const ALL: [Permanence; 3] = [Permanence::Permanent, Permanence::Plot, Permanence::Scene];

    pub const fn as_str(self) -> &'static str {
        match self {
            Permanence::Permanent => "permanent",
            Permanence::Plot => "plot",
            Permanence::Scene => "scene",
        }
    }

    pub fn from_str(s: &str) -> Option<Permanence> {
        match s {
            "permanent" => Some(Permanence::Permanent),
            "plot" => Some(Permanence::Plot),
            "scene" => Some(Permanence::Scene),
            _ => None,
        }
    }
}

impl Display for Permanence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single durable fact. Outlives the episode that produced it; `source_episode_id` is provenance.
#[derive(Clone, Debug)]
pub struct Fact {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub subject: String,
    pub fact: String,
    pub source_episode_id: Option<Uuid>,
    pub permanence: Permanence,
    pub superseded_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl Fact {
    pub fn new(
        adventure_id: Uuid,
        subject: impl Into<String>,
        fact: impl Into<String>,
        permanence: Permanence,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            subject: subject.into(),
            fact: fact.into(),
            source_episode_id: None,
            permanence,
            superseded_by: None,
            created_at: Utc::now(),
        }
    }

    pub fn from_episode(mut self, episode_id: Uuid) -> Self {
        self.source_episode_id = Some(episode_id);
        self
    }

    pub fn superseded_by(&self) -> Option<Uuid> {
        self.superseded_by
    }

    pub fn is_live(&self) -> bool {
        self.superseded_by.is_none()
    }

    pub fn render(facts: &[Fact]) -> Option<String> {
        let live: Vec<&Fact> = facts.iter().filter(|f| f.is_live()).collect();
        if live.is_empty() {
            return None;
        }

        let mut block = String::from("\nKnown facts:");
        for fact in live {
            block.push_str(&format!("\n- {}: {}", fact.subject, fact.fact));
        }
        Some(block)
    }
}
