use sqlx::SqlitePool;
use thiserror::Error;

use crate::backend::domain::memory::{Act, ActStatus, State};
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::insert_act;
use crate::backend::persistence::message_persistence::insert_message;
use crate::backend::persistence::state_persistence::upsert_state;
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
        goal: String,
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
        insert_act(
            &mut conn,
            &Act::new(adventure.id, 0, goal.as_str()).with_status(ActStatus::Active),
        )
        .await?;
        Ok(adventure)
    }

    pub async fn create_initial_messages(
        self,
        messages: &[Message],
    ) -> Result<(), AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        for message in messages {
            insert_message(&mut conn, &message.clone()).await?;
        }

        Ok(())
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
