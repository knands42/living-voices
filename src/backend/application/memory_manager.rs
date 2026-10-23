use sqlx::SqlitePool;
use thiserror::Error;

use crate::backend::PersistenceError;
use crate::backend::application::port::memory_port::{DistillError, MemoryDistiller};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::persistence::message_persistence::count_since_last_narration;

#[derive(Error, Debug)]
pub enum MemoryManagerError {
    #[error("failed to compress the memory {0}")]
    CompressionFailed(#[from] DistillError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
}

#[derive(Clone, Debug)]
pub struct MemoryManager<D: MemoryDistiller> {
    state: SharedState,
    pool: SqlitePool,
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

    pub async fn needs_compression(&self, threshold: u8) -> Result<bool, MemoryManagerError> {
        let adventure_id = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                return Ok(false);
            };
            adventure.id
        };

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let count = count_since_last_narration(&mut conn, adventure_id).await?;
        Ok(count >= threshold)
    }

    /// Compress every message not yet covered by the latest summary.
    pub async fn compress_and_update(&self) -> Result<(), MemoryManagerError> {
        let adventure_id = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                return Ok(());
            };
            adventure.id
        };

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        // build compression prompt
        // call distill
        // retrieve data in a given schema to update facts, act digest (if act closed), act running_digest, state, etc
        todo!()
    }
}
