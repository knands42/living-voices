use crate::ai::{AiError, MemoryUpdate};
use crate::backend::PersistenceError;
use crate::backend::application::state::{SharedState, StateError};
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TextManagerError {
    #[error("failed to compress the memory {0}")]
    CompressionFailed(#[from] AiError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
}

pub struct TextManager {
    state: SharedState,
    pool: SqlitePool,
}

impl TextManager {
    pub fn new(state: SharedState, pool: SqlitePool) -> Self {
        Self { state, pool }
    }

    pub fn compress(self) -> Result<(), TextManagerError> {
        {
            let mut guard = self.state.write()?;
        }

        Ok(())
    }
}
