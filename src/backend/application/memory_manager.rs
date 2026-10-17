use sqlx::SqlitePool;
use thiserror::Error;

use crate::ai::AiError;
use crate::backend::PersistenceError;
use crate::backend::application::state::{SharedState, StateError};

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
}

impl MemoryManager {
    pub fn new(state: SharedState, pool: SqlitePool) -> Self {
        Self { state, pool }
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

        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(PersistenceError::from)?;

        // TODO: find the latest `to_seq` covered for `adventure_id`, collect
        let _ = (&mut conn, adventure_id);

        Ok(())
    }
}
