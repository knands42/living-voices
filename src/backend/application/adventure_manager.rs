use sqlx::SqlitePool;
use thiserror::Error;

use crate::backend::domain::memory::State;
use crate::backend::persistence::memory_persistence::upsert_state;
use crate::backend::{
    PersistenceError,
    application::state::{SharedState, StateError},
    domain::{adventure::Adventure, character::Character},
    persistence::{
        adventure_persistence::insert_adventure, characters_persistence::insert_character,
        narrator_persistence::insert_narrator,
    },
};

#[derive(Error, Debug)]
pub enum AdventureManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
}

#[derive(Clone, Debug)]
pub struct AdventureManager {
    state: SharedState,
    pool: SqlitePool,
}

impl AdventureManager {
    pub fn new(state: SharedState, pool: SqlitePool) -> Self {
        Self { state, pool }
    }

    pub async fn create_adventure(
        &self,
        title: String,
        description: String,
        narrator_name: String,
        adventure_tone: &str,
    ) -> Result<Adventure, AdventureManagerError> {
        let mut adventure = Adventure::new(title, description, narrator_name, adventure_tone);
        adventure.characters.push(Character::player(adventure.id));
        {
            let mut guard = self.state.write()?;
            guard.set_adventure(adventure.clone());
        }

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        insert_adventure(&mut conn, &adventure).await?;
        insert_narrator(&mut conn, adventure.id, &adventure.narrator).await?;
        upsert_state(&mut conn, &State::new(adventure.id)).await?;
        for character in &adventure.characters {
            insert_character(&mut conn, Some(adventure.id), character).await?;
        }
        Ok(adventure)
    }

    pub fn current_adventure(&self) -> Option<Adventure> {
        self.state.read().ok()?.current_adventure.clone()
    }

    pub fn set_current_adventure(&self, adventure: Adventure) -> Result<(), AdventureManagerError> {
        let mut guard = self.state.write()?;
        guard.set_adventure(adventure);
        Ok(())
    }
}
