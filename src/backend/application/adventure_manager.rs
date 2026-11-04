use crate::backend::domain::adventure::AdventureStatus;
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::{Act, ActStatus, Fact, State};
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{
    activate_first_act, get_active_act, insert_default_act, upsert_act,
};
use crate::backend::persistence::adventure_persistence::{
    get_adventure_summaries, update_adventure, update_adventure_status,
};
use crate::backend::persistence::characters_persistence::insert_default_player_character;
use crate::backend::persistence::fact_persistence::{get_facts, insert_default_facts};
use crate::backend::persistence::lore_persistence::insert_default_lore;
use crate::backend::persistence::message_persistence::{fetch_last_n, insert_message};
use crate::backend::persistence::state_persistence::{get_state, insert_default_state};
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
    #[error("adventure is incomplete")]
    AdventureIncomplete(String),
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
        let default_state = insert_default_state(&mut conn, state_id, adventure_summary.id).await?;

        let fact_id = Uuid::new_v4();
        insert_default_facts(&mut conn, fact_id, adventure_summary.id).await?;

        let player_character_id = Uuid::new_v4();
        let default_player_character =
            insert_default_player_character(&mut conn, player_character_id, adventure_summary.id)
                .await?;

        let act_id = Uuid::new_v4();
        let default_act = insert_default_act(
            &mut conn,
            act_id,
            adventure_summary.id,
            0,
            Act::DEFAULT_ACT_GOAL,
            ActStatus::Unstarted.as_str(),
        )
        .await?;

        let lore_id = Uuid::new_v4();
        insert_default_lore(&mut conn, lore_id, adventure_summary.id).await?;

        let new_adventure = get_adventure(&mut conn, adventure_summary.id).await?;
        {
            let mut guard = self.state.write()?;
            guard.current_adventure = Some(new_adventure.clone());
            guard.current_act = Some(default_act);
            guard.current_state = Some(default_state);
            guard.player = Some(default_player_character);
        }

        Ok(new_adventure)
    }

    pub async fn update_status(
        &self,
        adventure_id: Uuid,
        status: AdventureStatus,
    ) -> Result<(), AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        update_adventure_status(&mut conn, adventure_id, status).await?;

        if let Ok(mut guard) = self.state.write() {
            if let Some(adventure) = &mut guard.current_adventure {
                if adventure.id == adventure_id {
                    adventure.status = status;
                }
            }
        }

        Ok(())
    }

    pub async fn update_adventure(
        &self,
        adventure: &Adventure,
    ) -> Result<Adventure, AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        update_adventure(&mut conn, adventure).await?;

        let updated = get_adventure(&mut conn, adventure.id).await?;

        if let Ok(mut guard) = self.state.write() {
            if let Some(current) = &guard.current_adventure {
                if current.id == updated.id {
                    guard.current_adventure = Some(updated.clone());
                }
            }
        }

        Ok(updated)
    }

    pub async fn create_initial_messages(
        &self,
        messages: &[Message],
    ) -> Result<(), AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        for message in messages {
            insert_message(&mut conn, &message.clone()).await?;
        }

        Ok(())
    }

    pub async fn upsert_act(&self, act: &Act) -> Result<(), AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;

        let upserted_act = upsert_act(&mut conn, act).await?;

        if upserted_act.status == ActStatus::Active {
            let mut guard = self.state.write()?;
            guard.current_act = Some(upserted_act.clone());
        }

        Ok(())
    }

    pub async fn start_adventure(&self, adventure_id: Uuid) -> Result<(), AdventureManagerError> {
        self.validate_adventure_before_start(adventure_id).await?;

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let current_act = activate_first_act(&mut conn, adventure_id).await?;
        let current_state = get_state(&mut conn, adventure_id).await?;
        let current_facts = get_facts(&mut conn, adventure_id).await?;
        let current_messages = fetch_last_n(&mut conn, adventure_id, 50).await?;

        let adventure = get_adventure(&mut conn, adventure_id).await?;
        let player = adventure.characters.iter().find(|c| c.is_player).cloned();
        {
            let mut guard = self.state.write()?;
            guard.current_adventure = Some(adventure);
            guard.current_act = Some(current_act);
            guard.current_state = current_state;
            guard.current_facts = current_facts;
            guard.player = player;
        }

        Ok(())
    }

    async fn validate_adventure_before_start(
        &self,
        adventure_id: Uuid,
    ) -> Result<(), AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;

        let act = get_active_act(&mut conn, adventure_id)
            .await?
            .ok_or_else(|| {
                AdventureManagerError::AdventureIncomplete("no active act found".into())
            })?;

        if !act.is_valid_act() {
            return Err(AdventureManagerError::AdventureIncomplete(
                "act is incomplete".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn get_messages(
        &self,
        adventure_id: Uuid,
        limit: usize,
    ) -> Result<Vec<Message>, AdventureManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        Ok(fetch_last_n(&mut conn, adventure_id, limit as u16).await?)
    }
}

impl AdventureManager {
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

    pub fn current_snapshot(&self) -> MemorySnapshot {
        match self.state.read() {
            Ok(guard) => MemorySnapshot::new(
                guard.current_adventure.clone(),
                guard.current_act.clone(),
                guard.current_state.clone(),
                guard.current_facts.clone(),
                guard.player.clone(),
            ),
            Err(_) => MemorySnapshot::default(),
        }
    }
}

#[derive(Default)]
pub struct MemorySnapshot {
    pub current_adventure: Option<Adventure>,
    pub current_act: Option<Act>,
    pub current_state: Option<State>,
    pub current_facts: Vec<Fact>,
    pub player: Option<Character>,
}

impl MemorySnapshot {
    fn new(
        current_adventure: Option<Adventure>,
        current_act: Option<Act>,
        current_state: Option<State>,
        current_facts: Vec<Fact>,
        player: Option<Character>,
    ) -> Self {
        Self {
            current_adventure,
            current_act,
            current_state,
            current_facts,
            player,
        }
    }
}
