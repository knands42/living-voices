//! Creates a draft character, persists it, updates local adventure state,
use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::backend::application::character_gallery::CharacterGallery;
use crate::backend::application::context::{ContextBuilder, ContextBuilderError};
use crate::backend::application::port::chat_port::{ChatResponseError, PromptPort};
use crate::backend::application::port::image_port::{ImageError, ImagePort, ShotReference};
use crate::backend::domain::character::ImageFraming;
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
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    DatabaseError(#[from] PersistenceError),
    #[error("prompt context error: {0}")]
    Context(#[from] ContextBuilderError),
    #[error(transparent)]
    Prompt(#[from] ChatResponseError),
    #[error(transparent)]
    Image(#[from] ImageError),
}

#[derive(Clone, Copy)]
enum ImagePromptKind {
    Identity,
    Outfit,
    Pose,
}

impl ImagePromptKind {
    fn label(self) -> &'static str {
        match self {
            ImagePromptKind::Identity => "identity",
            ImagePromptKind::Outfit => "outfit",
            ImagePromptKind::Pose => "pose",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CharacterManager<PROMPT: PromptPort, IMAGE: ImagePort> {
    state: SharedState,
    pool: SqlitePool,
    prompt_handler: PROMPT,
    image_handler: IMAGE,
    gallery: CharacterGallery,
}

impl<PROMPT: PromptPort, IMAGE: ImagePort> CharacterManager<PROMPT, IMAGE> {
    pub fn new(
        state: SharedState,
        pool: SqlitePool,
        prompt_handler: PROMPT,
        image_handler: IMAGE,
        gallery: CharacterGallery,
    ) -> Self {
        CharacterManager {
            state,
            pool,
            prompt_handler,
            image_handler,
            gallery,
        }
    }

    pub fn gallery(&self) -> &CharacterGallery {
        &self.gallery
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
            &[character.name.as_str()],
            &[],
        )
        .await?;

        log::info!(
            "character created: {} ({}) in adventure {}",
            character.name,
            character.id,
            character.adventure_id
        );
        Ok(())
    }

    /// Opens a page's worth of character: persisted straight away so every field
    /// saves on its own, returned so the page never reads it back.
    pub async fn create_draft(
        &self,
        adventure_id: Uuid,
    ) -> Result<Character, CharacterManagerError> {
        let character = Character::draft(adventure_id);
        self.create_character(character.clone()).await?;
        Ok(character)
    }

    pub async fn update_character(
        &self,
        character: Character,
    ) -> Result<(), CharacterManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        update_character(&mut conn, &character).await?;

        {
            let mut guard = self.state.write()?;
            if let Some(adventure) = &mut guard.current_adventure
                && let Some(existing) = adventure
                    .characters
                    .iter_mut()
                    .find(|c| c.id == character.id)
            {
                *existing = character.clone();
            }
            if let Some(player) = &mut guard.player
                && player.id == character.id
            {
                *player = character.clone();
            }
        }

        log::debug!("character updated: {} ({})", character.name, character.id);
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

    pub async fn create_character_image_portrait(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        let reference = self.get_latest_full_body_image(character).await?;
        self.create_character_image(character, ImageFraming::Portrait, reference.as_deref())
            .await
    }

    pub async fn create_character_image_full_body(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image(character, ImageFraming::FullBody, None)
            .await
    }

    pub async fn create_character_image_pose(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        let reference = self.get_latest_full_body_image(character).await?;
        self.create_character_image(character, ImageFraming::FullBodyPose, reference.as_deref())
            .await
    }

    async fn get_latest_full_body_image(
        &self,
        character: &Character,
    ) -> Result<Option<PathBuf>, CharacterManagerError> {
        let title = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                log::warn!(
                    "no loaded adventure; skipping the image of {}",
                    character.name
                );
                return Ok(None);
            };

            adventure.title.clone()
        };

        if let Some(path) = self
            .gallery
            .latest(&title, character, ImageFraming::FullBody)
        {
            return Ok(Some(path));
        }

        Ok(None)
    }

    async fn create_character_image(
        &self,
        character: &Character,
        framing: ImageFraming,
        reference: Option<&Path>,
    ) -> Result<(), CharacterManagerError> {
        let Some(prompt) = ContextBuilder::compose_character_image_prompt(character, framing)
        else {
            log::debug!("no appearance to compose an image of {}", character.name);
            return Ok(());
        };

        let destination = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                log::warn!(
                    "no loaded adventure; skipping the image of {}",
                    character.name
                );
                return Ok(());
            };

            self.gallery.prefix(&adventure.title, character, framing)
        };

        self.image_handler
            .generate_character_image(
                &prompt,
                &destination,
                reference.map(|path| ShotReference { path, framing }),
            )
            .await?;

        log::info!("generated an image of {} at {destination}", character.name);
        Ok(())
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
                ContextBuilder::build_identity_char_prompt(&adventure, character, conditions)?
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

        log::debug!("generating {} prompt for {}", kind.label(), character.name);
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

        self.update_image_prompt_shared_state(character.id, kind, &generated_prompt)?;
        log::info!("stored {} prompt for {}", kind.label(), character.name);

        Ok(())
    }

    fn update_image_prompt_shared_state(
        &self,
        character_id: Uuid,
        kind: ImagePromptKind,
        prompt: &str,
    ) -> Result<(), StateError> {
        let mut guard = self.state.write()?;

        let mut stored = false;
        if let Some(adventure) = &mut guard.current_adventure
            && let Some(character) = adventure
                .characters
                .iter_mut()
                .find(|character| character.id == character_id)
        {
            set_image_prompt(character, kind, prompt);
            stored = true;
        }

        if let Some(player) = &mut guard.player
            && player.id == character_id
        {
            set_image_prompt(player, kind, prompt);
            stored = true;
        }

        if !stored {
            log::debug!(
                "{} prompt generated for {character_id} but the character is not in the loaded adventure",
                kind.label()
            );
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
