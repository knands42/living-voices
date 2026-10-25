use std::fmt::Display;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// How long a fact is expected to hold. Mirrors the vocabulary the DB enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Permanence {
    Permanent,
    #[default]
    Plot,
}

impl Permanence {
    pub const fn as_str(self) -> &'static str {
        match self {
            Permanence::Permanent => "permanent",
            Permanence::Plot => "plot",
        }
    }

    pub fn from_str(s: &str) -> Option<Permanence> {
        match s {
            "permanent" => Some(Permanence::Permanent),
            "plot" => Some(Permanence::Plot),
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
    pub act_id: Uuid,
    pub permanence: Permanence,
    pub created_at: DateTime<Utc>,
}

impl Fact {
    pub fn new(
        adventure_id: Uuid,
        subject: impl Into<String>,
        fact: impl Into<String>,
        act_id: Uuid,
        permanence: Permanence,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            subject: subject.into(),
            fact: fact.into(),
            act_id,
            permanence,
            created_at: Utc::now(),
        }
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
