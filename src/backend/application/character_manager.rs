use sqlx::SqlitePool;
use thiserror::Error;

use crate::backend::application::port::chat_port::PromptHandler;
use crate::backend::{
    PersistenceError,
    application::state::{SharedState, StateError},
    domain::character::Character,
    persistence::characters_persistence::insert_character,
};

#[derive(Error, Debug)]
pub enum CharacterManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
}

#[derive(Clone, Debug)]
pub struct CharacterManager<PROMPT: PromptHandler> {
    state: SharedState,
    pool: SqlitePool,
    prompt_handler: PROMPT,
}

impl<PROMPT: PromptHandler> CharacterManager<PROMPT> {
    pub fn new(state: SharedState, pool: SqlitePool, prompt_handler: PROMPT) -> Self {
        CharacterManager {
            state,
            pool,
            prompt_handler,
        }
    }

    pub async fn create_character(
        &self,
        character: Character,
    ) -> Result<(), CharacterManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        insert_character(&mut conn, character.adventure_id, &character).await?;

        {
            let mut guard = self.state.write()?;
            if let Some(adventure) = &mut guard.current_adventure {
                adventure.add_character(character);
            }
        }

        Ok(())
    }

    pub async fn create_character_portrait_image(
        &self,
        _character_appearance: &str,
    ) -> Result<(), CharacterManagerError> {
        // TODO: build the character portrait image prompt, call the PromptHandler,
        // and persist the result.

        Ok(())
    }

    pub async fn create_character_full_body_image(
        &self,
        _character_appearance: &str,
    ) -> Result<(), CharacterManagerError> {
        // TODO: build the character full-body image prompt, call the PromptHandler,
        // and persist the result.

        Ok(())
    }
}
