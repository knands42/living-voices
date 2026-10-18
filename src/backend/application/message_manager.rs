use crate::backend::PersistenceError;
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::message::Message;
use crate::backend::persistence::message_persistence::insert_message;
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
}

#[derive(Clone, Debug)]
pub struct MessageManager {
    state: SharedState,
    pool: SqlitePool,
}

impl MessageManager {
    pub fn new(state: SharedState, pool: SqlitePool) -> Self {
        Self { state, pool }
    }

    pub async fn create_message(self, message: Message) -> Result<(), MessageManagerError> {
        {
            let _unused = self.state.write()?;
        }

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        insert_message(&mut conn, &message).await?;
        Ok(())
    }
}
