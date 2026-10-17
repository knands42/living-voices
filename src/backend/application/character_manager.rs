use sqlx::SqlitePool;
use thiserror::Error;

use crate::backend::{
    PersistenceError,
    application::state::{SharedState, StateError},
    domain::character::Character,
    persistence::characters::insert_character,
};

#[derive(Error, Debug)]
pub enum CharacterManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
}

#[derive(Clone, Debug)]
pub struct CharacterManager {
    state: SharedState,
    pool: SqlitePool,
}

impl CharacterManager {
    pub fn new(state: SharedState, pool: SqlitePool) -> Self {
        CharacterManager { state, pool }
    }

    pub async fn create_character(
        &self,
        character: Character,
    ) -> Result<(), CharacterManagerError> {
        {
            let mut guard = self.state.write()?;
            guard.set_character(character.clone());
        }

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        insert_character(&mut conn, character.adventure_id, &character).await?;
        Ok(())
    }
}
