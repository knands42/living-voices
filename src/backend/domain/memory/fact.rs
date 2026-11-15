use serde::{Deserialize, Serialize};

/// A single atomic fact inside an adventure's fact snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub subject: String,
    pub fact: String,
}

impl Fact {
    pub fn new(subject: impl Into<String>, fact: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            fact: fact.into(),
        }
    }

    pub fn render(facts: &[Fact]) -> Option<String> {
        if facts.is_empty() {
            return None;
        }

        let mut block = String::new();
        for fact in facts {
            block.push_str(&format!("\n- {}: {}", fact.subject, fact.fact));
        }
        Some(block)
    }
}
