use std::fmt::Display;

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ActStatus {
    #[default]
    Unstarted,
    Active,
    Resolved,
    Failed,
    Bypassed,
}

impl ActStatus {
    pub const ALL: [ActStatus; 5] = [
        ActStatus::Unstarted,
        ActStatus::Active,
        ActStatus::Resolved,
        ActStatus::Failed,
        ActStatus::Bypassed,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            ActStatus::Unstarted => "unstarted",
            ActStatus::Active => "active",
            ActStatus::Resolved => "resolved",
            ActStatus::Failed => "failed",
            ActStatus::Bypassed => "bypassed",
        }
    }

    pub fn from_str(s: &str) -> Option<ActStatus> {
        match s {
            "unstarted" => Some(ActStatus::Unstarted),
            "active" => Some(ActStatus::Active),
            "resolved" => Some(ActStatus::Resolved),
            "failed" => Some(ActStatus::Failed),
            "bypassed" => Some(ActStatus::Bypassed),
            _ => None,
        }
    }
}

impl Display for ActStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A story movement: the only story unit, lasting roughly 200-600 messages.
#[derive(Clone, Debug)]
pub struct Act {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub ordinal: i64,
    pub goal: String,
    pub trigger: Option<String>,
    pub status: ActStatus,
    pub index_line: Option<String>,
    pub digest: Option<String>,
    pub running_digest: Option<String>,
    pub from_seq: Option<u64>,
    pub to_seq: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Act {
    pub fn new(adventure_id: Uuid, ordinal: i64, goal: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            adventure_id,
            ordinal,
            goal: goal.into(),
            trigger: None,
            status: ActStatus::default(),
            index_line: None,
            digest: None,
            running_digest: None,
            from_seq: None,
            to_seq: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_trigger(mut self, trigger: impl Into<String>) -> Self {
        self.trigger = Some(trigger.into());
        self
    }

    pub fn with_status(mut self, status: ActStatus) -> Self {
        self.status = status;
        self
    }

    pub fn close(
        &mut self,
        index_line: impl Into<String>,
        digest: impl Into<String>,
        from_seq: u64,
        to_seq: u64,
    ) {
        self.index_line = Some(index_line.into());
        self.digest = Some(digest.into());
        self.from_seq = Some(from_seq);
        self.to_seq = Some(to_seq);
    }
}
