use sqlx::SqlitePool;
use thiserror::Error;

use crate::ai::{AiError, ChatMessage, MemoryUpdate, OpenAiCompatibleClient};
use crate::backend::PersistenceError;
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::memory::{Fact, Summary};
use crate::backend::domain::message::Message;
use crate::backend::persistence::memory_persistence::{insert_facts, insert_summary, last_covered};
use crate::backend::persistence::message_persistence::fetch_since;
use crate::config::TextModel;

#[derive(Error, Debug)]
pub enum MemoryManagerError {
    #[error("failed to compress the memory {0}")]
    CompressionFailed(#[from] AiError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
}

#[derive(Clone, Debug)]
pub struct MemoryManager {
    state: SharedState,
    pool: SqlitePool,
    /// OpenAI-compatible endpoint used for the distillation calls.
    base_url: String,
    model: String,
}

impl MemoryManager {
    pub fn new(state: SharedState, pool: SqlitePool, model: TextModel) -> Self {
        Self {
            state,
            pool,
            base_url: model.base_url,
            model: model.model,
        }
    }

    /// Compress every message not yet covered by the latest summary.
    pub async fn compress(&self) -> Result<(), MemoryManagerError> {
        let adventure_id = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                return Ok(());
            };
            adventure.id
        };

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;

        // If nothing, skip
        let after_seq = match last_covered(&mut conn, adventure_id).await {
            Some(covered) => covered as i64,
            None => 0,
        };

        let messages = fetch_since(&mut conn, adventure_id, after_seq).await?;
        let Some(to_seq) = last_seq(&messages) else {
            return Ok(());
        };
        let from_seq = after_seq + 1;

        let history = to_history(&messages);
        let client = OpenAiCompatibleClient::new(&self.base_url)?;
        let update = MemoryUpdate::distill(&client, &self.model, &history).await?;

        let summary = Summary::new(adventure_id, update.summary, from_seq, to_seq);
        insert_summary(&mut conn, &summary).await?;

        let facts: Vec<Fact> = update
            .facts
            .into_iter()
            .map(|text| Fact::new(adventure_id, summary.id, text))
            .collect();
        insert_facts(&mut conn, &facts).await?;

        Ok(())
    }
}

/// `seq` of the newest message in the batch, or `None` when the batch is empty.
fn last_seq(messages: &[Message]) -> Option<i64> {
    messages.last().and_then(|m| m.seq).map(|s| s as i64)
}

/// Render the message batch as the conversation handed to the memory model.
fn to_history(messages: &[Message]) -> Vec<ChatMessage> {
    messages
        .iter()
        .map(|m| {
            let mut line = format!("[{}] {}: {}", m.kind, m.speaker_name, m.text);
            // TODO: Improve the order here on formatting the message: `[<character> say] to [<target>]: <message>`
            if let Some(target) = &m.target_name {
                line.push_str(&format!(" (to {target})"));
            }
            ChatMessage::user(line)
        })
        .collect()
}
