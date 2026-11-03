use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// How long a fact is expected to hold. Mirrors the vocabulary the DB enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

/// A single atomic fact inside an adventure's fact snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub subject: String,
    pub fact: String,
    pub permanence: Permanence,
}

impl Fact {
    pub fn new(
        subject: impl Into<String>,
        fact: impl Into<String>,
        permanence: Permanence,
    ) -> Self {
        Self {
            subject: subject.into(),
            fact: fact.into(),
            permanence,
        }
    }

    pub fn render(facts: &[Fact]) -> Option<String> {
        if facts.is_empty() {
            return None;
        }

        let mut block = String::new();
        for fact in facts {
            block.push_str(&format!(
                "\n- {}: {} -> {}",
                fact.subject, fact.fact, fact.permanence
            ));
        }
        Some(block)
    }
}
