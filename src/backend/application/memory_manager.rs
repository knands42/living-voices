use sqlx::SqlitePool;
use thiserror::Error;

use crate::backend::PersistenceError;
use crate::backend::application::port::memory_port::{
    DistillError, DistilledMemory, MemoryDistiller,
};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::memory::{Fact, Summary};
use crate::backend::persistence::memory_persistence::{insert_facts, insert_summary, last_covered};
use crate::backend::persistence::message_persistence::fetch_since;

#[derive(Error, Debug)]
pub enum MemoryManagerError {
    #[error("failed to compress the memory {0}")]
    CompressionFailed(#[from] DistillError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
}

#[derive(Clone)]
pub struct MemoryManager<D: MemoryDistiller> {
    state: SharedState,
    pool: SqlitePool,
    /// The memory port, chosen at compile time. Never names an `ai` type.
    distiller: D,
}

impl<D: MemoryDistiller> MemoryManager<D> {
    pub fn new(state: SharedState, pool: SqlitePool, distiller: D) -> Self {
        Self {
            state,
            pool,
            distiller,
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

        let after_seq = last_covered(&mut conn, adventure_id)
            .await
            .unwrap_or_else(|| 0);
        let messages = fetch_since(&mut conn, adventure_id, after_seq).await?;
        let to_seq = messages.last().and_then(|m| m.seq).unwrap_or(0);
        if to_seq == 0 {
            return Ok(());
        }
        let from_seq = after_seq + 1;

        let DistilledMemory { facts, summary } = self.distiller.distill(&messages).await?;
        let summary = Summary::new(adventure_id, summary, from_seq, to_seq);
        insert_summary(&mut conn, &summary).await?;

        let facts: Vec<Fact> = facts
            .into_iter()
            .map(|text| Fact::new(adventure_id, summary.id, text))
            .collect();
        insert_facts(&mut conn, &facts).await?;

        Ok(())
    }
}
