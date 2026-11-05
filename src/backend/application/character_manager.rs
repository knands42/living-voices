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
    persistence::adventure_persistence::get_adventure,
    persistence::characters_persistence::{
        insert_character, update_appearance_current_outfit, update_appearance_current_pose,
        update_appearance_identity_prompt, update_character,
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
    Identity,
    Outfit,
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

    pub async fn create_character_identity_prompt(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image_prompt(character, ImagePromptKind::Identity)
            .await
    }

    pub async fn create_character_outfit_prompt(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image_prompt(character, ImagePromptKind::Outfit)
            .await
    }

    pub async fn create_character_pose_prompt(
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

        let prompt = match kind {
            ImagePromptKind::Identity => {
                (ContextBuilder::build_identity_char_prompt(&adventure, character, conditions)?)
            }
            ImagePromptKind::Outfit => {
                let messages = fetch_last_n(&mut conn, character.adventure_id, 50).await?;
                ContextBuilder::build_outfit_char_prompt(
                    &adventure, character, conditions, &messages,
                )?
            }
            ImagePromptKind::Pose => {
                let messages = fetch_last_n(&mut conn, character.adventure_id, 50).await?;
                ContextBuilder::build_pose_body_prompt(
                    &adventure, character, conditions, &messages,
                )?
            }
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
            ImagePromptKind::Identity => {
                update_appearance_identity_prompt(&mut conn, character.id, &generated_prompt)
                    .await?
            }
            ImagePromptKind::Outfit => {
                update_appearance_current_outfit(&mut conn, character.id, &generated_prompt).await?
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
        ImagePromptKind::Identity => &mut character.appearance_identity_prompt,
        ImagePromptKind::Outfit => &mut character.appearance_current_outfit,
        ImagePromptKind::Pose => &mut character.appearance_current_pose,
    };
    *slot = Some(prompt.to_string());
}
