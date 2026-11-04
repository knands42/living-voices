use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::backend::application::context::{ContextBuilder, ContextBuilderError};
use crate::backend::application::port::chat_port::{ChatResponseError, PromptHandler};
use crate::backend::persistence::state_persistence::update_character_presence;
use crate::backend::{
    PersistenceError,
    application::state::{SharedState, StateError},
    domain::character::Character,
    domain::memory::CharacterCondition,
    domain::message::Message,
    persistence::adventure_persistence::get_adventure,
    persistence::characters_persistence::{
        insert_character, update_appearance_current_pose, update_appearance_full_body_prompt,
        update_appearance_portrait_prompt, update_character,
    },
    persistence::message_persistence::fetch_last_n,
    persistence::state_persistence::get_state,
};

#[derive(Error, Debug)]
pub enum CharacterManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
    #[error("prompt context error: {0}")]
    Context(#[from] ContextBuilderError),
    #[error("prompt provider failed: {0}")]
    Prompt(#[from] ChatResponseError),
}

#[derive(Clone, Copy)]
enum ImagePromptKind {
    Portrait,
    FullBody,
    Pose,
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
                adventure.add_character(character.clone());
            }
        }

        update_character_presence(
            &mut conn,
            character.adventure_id,
            &vec![character.name.as_str()],
            &vec![],
        )
        .await?;

        Ok(())
    }

    pub async fn update_character(
        &self,
        character: Character,
    ) -> Result<(), CharacterManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        update_character(&mut conn, &character).await?;

        {
            let mut guard = self.state.write()?;
            if let Some(adventure) = &mut guard.current_adventure {
                if let Some(existing) = adventure
                    .characters
                    .iter_mut()
                    .find(|c| c.id == character.id)
                {
                    *existing = character.clone();
                }
            }
            if let Some(player) = &mut guard.player {
                if player.id == character.id {
                    *player = character.clone();
                }
            }
        }

        Ok(())
    }

    pub async fn create_character_portrait_image(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image_prompt(character, ImagePromptKind::Portrait)
            .await
    }

    pub async fn create_character_full_body_image(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image_prompt(character, ImagePromptKind::FullBody)
            .await
    }

    pub async fn create_character_pose_image(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image_prompt(character, ImagePromptKind::Pose)
            .await
    }

    async fn create_character_image_prompt(
        &self,
        character: &Character,
        kind: ImagePromptKind,
    ) -> Result<(), CharacterManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;

        let adventure = get_adventure(&mut conn, character.adventure_id).await?;
        let conditions: Vec<CharacterCondition> = get_state(&mut conn, character.adventure_id)
            .await?
            .map(|state| {
                CharacterCondition::for_character(&state.character_conditions, &character.name)
                    .into_iter()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let messages: Vec<Message> = match kind {
            ImagePromptKind::Pose => fetch_last_n(&mut conn, character.adventure_id, 50).await?,
            ImagePromptKind::Portrait | ImagePromptKind::FullBody => Vec::new(),
        };

        let prompt = match kind {
            ImagePromptKind::Portrait => {
                ContextBuilder::build_portrait_char_prompt(&adventure, character, conditions)?
            }
            ImagePromptKind::FullBody => {
                ContextBuilder::build_full_body_char_prompt(&adventure, character, conditions)?
            }
            ImagePromptKind::Pose => ContextBuilder::build_pose_body_prompt(
                &adventure, character, conditions, &messages,
            )?,
        };

        // Do not hold the database connection while the model call is in flight.
        drop(conn);

        let generated_prompt = self
            .prompt_handler
            .handle_prompt(&prompt)
            .await?
            .trim()
            .to_string();

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        match kind {
            ImagePromptKind::Portrait => {
                update_appearance_portrait_prompt(&mut conn, character.id, &generated_prompt)
                    .await?
            }
            ImagePromptKind::FullBody => {
                update_appearance_full_body_prompt(&mut conn, character.id, &generated_prompt)
                    .await?
            }
            ImagePromptKind::Pose => {
                update_appearance_current_pose(&mut conn, character.id, &generated_prompt).await?
            }
        }
        drop(conn);

        self.store_image_prompt(character.id, kind, &generated_prompt)?;

        Ok(())
    }

    fn store_image_prompt(
        &self,
        character_id: Uuid,
        kind: ImagePromptKind,
        prompt: &str,
    ) -> Result<(), StateError> {
        let mut guard = self.state.write()?;

        if let Some(adventure) = &mut guard.current_adventure {
            if let Some(character) = adventure
                .characters
                .iter_mut()
                .find(|character| character.id == character_id)
            {
                set_image_prompt(character, kind, prompt);
            }
        }

        if let Some(player) = &mut guard.player {
            if player.id == character_id {
                set_image_prompt(player, kind, prompt);
            }
        }

        Ok(())
    }
}

fn set_image_prompt(character: &mut Character, kind: ImagePromptKind, prompt: &str) {
    let slot = match kind {
        ImagePromptKind::Portrait => &mut character.appearance_portrait_prompt,
        ImagePromptKind::FullBody => &mut character.appearance_full_body_prompt,
        ImagePromptKind::Pose => &mut character.appearance_current_pose,
    };
    *slot = Some(prompt.to_string());
}
