use crate::backend::domain::memory::{Act, ActStatus, State};
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::insert_act;
use crate::backend::persistence::message_persistence::insert_message;
use crate::backend::persistence::state_persistence::upsert_state;
use crate::backend::{
    PersistenceError,
    application::state::{SharedState, StateError},
    domain::{
        adventure::{Adventure, AdventureSummary},
        character::Character,
    },
    persistence::{
        adventure_persistence::{get_adventures, insert_adventure},
        characters_persistence::insert_character,
        narrator_persistence::insert_default_narrator,
    },
};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

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
        headline: String,
        tone: String,
    ) -> Result<Adventure, AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;

        let adventure_id = Uuid::new_v4();
        let adventure_summary = insert_adventure(
            &mut conn,
            adventure_id,
            title.as_str(),
            headline.as_str(),
            tone.as_str(),
            vec![],
        )
        .await?;

        let narrator_id = Uuid::new_v4();
        insert_default_narrator(&mut conn, narrator_id, adventure_id).await?;


        upsert_state(&mut conn, &State::new(adventure.id)).await?;
        // default character: player
        insert_character(&mut conn, Some(adventure.id), character).await?;
        insert_act(
            &mut conn,
            &Act::new(adventure.id, 0, goal.as_str()).with_status(ActStatus::Active),
        )
        .await?;

        {
            let mut guard = self.state.write()?;
            guard.set_adventure(adventure.clone());
        }

        Ok(created_adventure)
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

    pub async fn get_adventures_summaries(&self) -> Result<Vec<Adventure>, AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        Ok(get_adventures(&mut conn).await?)
    }

    pub fn current_adventure(&self) -> Option<Adventure> {
        self.state.read().ok()?.current_adventure.clone()
        // TODO: Should I set the state for it as well?
    }

    pub fn set_current_adventure(&self, adventure: Adventure) -> Result<(), AdventureManagerError> {
        let mut guard = self.state.write()?;
        guard.set_adventure(adventure);
        Ok(())
    }
}
