use crate::backend::domain::memory::ActStatus;
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{activate_first_act, insert_default_act};
use crate::backend::persistence::adventure_persistence::get_adventure_summaries;
use crate::backend::persistence::characters_persistence::insert_default_player_character;
use crate::backend::persistence::lore_persistence::insert_default_lore;
use crate::backend::persistence::message_persistence::insert_message;
use crate::backend::persistence::state_persistence::insert_default_state;
use crate::backend::{
    PersistenceError,
    application::state::{SharedState, StateError},
    domain::adventure::{Adventure, AdventureSummary},
    persistence::{
        adventure_persistence::{get_adventure, insert_adventure},
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
    #[error("no adventure is currently active")]
    NoAdventureActive,
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

        let state_id = Uuid::new_v4();
        insert_default_state(&mut conn, state_id, adventure_summary.id).await?;

        let player_character_id = Uuid::new_v4();
        insert_default_player_character(&mut conn, player_character_id, adventure_summary.id)
            .await?;

        let act_id = Uuid::new_v4();
        let act_goal = "PENDING GOAL";
        insert_default_act(
            &mut conn,
            act_id,
            adventure_summary.id,
            0,
            act_goal,
            ActStatus::Unstarted.as_str(),
        )
        .await?;

        let lore_id = Uuid::new_v4();
        insert_default_lore(&mut conn, lore_id, adventure_summary.id).await?;

        let new_adventure = get_adventure(&mut conn, adventure_summary.id).await?;
        {
            let mut guard = self.state.write()?;
            guard.current_adventure = Some(new_adventure.clone());
        }

        Ok(new_adventure)
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

    pub async fn get_adventures_summaries(
        &self,
    ) -> Result<Vec<AdventureSummary>, AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        Ok(get_adventure_summaries(&mut conn).await?)
    }

    pub async fn get_adventure(
        &self,
        adventure_id: Uuid,
    ) -> Result<Adventure, AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        Ok(get_adventure(&mut conn, adventure_id).await?)
    }

    pub fn current_adventure(&self) -> Option<Adventure> {
        self.state.read().ok()?.current_adventure.clone()
    }

    pub async fn start_adventure(&self, adventure: Adventure) -> Result<(), AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        activate_first_act(&mut conn, adventure.id).await?;
        {
            let mut guard = self.state.write()?;
            guard.current_adventure = Some(adventure);
        }

        Ok(())
    }
}
