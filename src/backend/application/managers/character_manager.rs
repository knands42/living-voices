//! Creates a draft character, persists it, updates local adventure state,
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::backend::application::character_gallery::CharacterGallery;
use crate::backend::application::context::{ContextBuilder, ContextBuilderError};
use crate::backend::application::port::chat_port::{ChatResponseError, PromptPort};
use crate::backend::application::port::image_port::{ImageError, ImagePort};
use crate::backend::domain::character::ImageFraming;
use crate::backend::domain::lora::{Lora, LoraKind};
use crate::backend::persistence::state_persistence::update_character_presence;
use crate::backend::{
    PersistenceError,
    application::state::{SharedState, StateError},
    domain::character::Character,
    domain::memory::CharacterCondition,
    persistence::adventure_persistence::get_adventure,
    persistence::characters_persistence::{
        delete_character, insert_character, update_appearance_current_body,
        update_appearance_current_outfit, update_appearance_current_pose, update_character,
    },
    persistence::message_persistence::fetch_last_n,
    persistence::state_persistence::get_state,
};
use crate::config::ConfigHandle;

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
    #[error("the player character cannot be deleted")]
    PlayerCannotBeDeleted,
}

#[derive(Clone, Copy)]
enum ImagePromptKind {
    Body,
    Outfit,
    Pose,
}

impl ImagePromptKind {
    fn label(self) -> &'static str {
        match self {
            ImagePromptKind::Body => "body",
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
    config: ConfigHandle,
}

impl<PROMPT: PromptPort, IMAGE: ImagePort> CharacterManager<PROMPT, IMAGE> {
    pub fn new(
        state: SharedState,
        pool: SqlitePool,
        prompt_handler: PROMPT,
        image_handler: IMAGE,
        gallery: CharacterGallery,
        config: ConfigHandle,
    ) -> Self {
        CharacterManager {
            state,
            pool,
            prompt_handler,
            image_handler,
            gallery,
            config,
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

    pub async fn delete_character(
        &self,
        character: Character,
    ) -> Result<(), CharacterManagerError> {
        if character.is_player {
            return Err(CharacterManagerError::PlayerCannotBeDeleted);
        }

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        delete_character(&mut conn, character.id).await?;
        update_character_presence(
            &mut conn,
            character.adventure_id,
            &[],
            &[character.name.as_str()],
        )
        .await?;

        {
            let mut guard = self.state.write()?;
            if let Some(adventure) = &mut guard.current_adventure {
                adventure
                    .characters
                    .retain(|existing| existing.id != character.id);
            }
            if let Some(state) = &mut guard.current_state {
                state
                    .characters_present
                    .retain(|name| name != &character.name);
            }
        }

        log::info!("character deleted: {} ({})", character.name, character.id);
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

    /// Copies the lora file the author picked into the engine's lora directory
    pub async fn stage_lora_adapter(
        &self,
        source: &str,
        scale: f32,
        kind: LoraKind,
    ) -> Option<Lora> {
        let source = std::path::Path::new(source.trim());
        if source.as_os_str().is_empty() {
            return None;
        }

        let dir = self.adapter_dir(kind);
        let candidate = if source.is_absolute() {
            source.to_path_buf()
        } else {
            dir.join(source)
        };

        if candidate.is_file()
            && candidate.parent() == Some(dir.as_path())
            && let Some(name) = candidate.file_name()
        {
            return Some(Lora {
                name: name.to_string_lossy().into_owned(),
                scale,
                kind,
            });
        }

        if !source.is_file() {
            log::warn!("adapter {} is missing; nothing to stage", source.display());
            return None;
        }

        let name = adapter_name(source, &dir);
        let target = dir.join(&name);
        if let Err(err) = tokio::fs::copy(source, &target).await {
            log::warn!("could not stage {}: {err}", source.display());
            return None;
        }

        Some(Lora { name, scale, kind })
    }

    fn adapter_dir(&self, kind: LoraKind) -> std::path::PathBuf {
        match kind {
            LoraKind::Image => self.config.settings().models.image.loras_dir(),
            LoraKind::Text => self.config.settings().models.chat.loras_dir(),
        }
    }

    pub async fn create_character_body_prompt(
        &self,
        character: &Character,
    ) -> Result<Character, CharacterManagerError> {
        self.create_character_image_prompt(character, ImagePromptKind::Body)
            .await
    }

    pub async fn create_character_outfit_prompt(
        &self,
        character: &Character,
    ) -> Result<Character, CharacterManagerError> {
        self.create_character_image_prompt(character, ImagePromptKind::Outfit)
            .await
    }

    pub async fn create_character_pose_prompt(
        &self,
        character: &Character,
    ) -> Result<Character, CharacterManagerError> {
        self.create_character_image_prompt(character, ImagePromptKind::Pose)
            .await
    }

    pub async fn create_character_image_portrait(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image(character, ImageFraming::Portrait)
            .await
    }

    pub async fn create_character_image_full_body(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image(character, ImageFraming::FullBody)
            .await
    }

    pub async fn create_character_image_pose(
        &self,
        character: &Character,
    ) -> Result<(), CharacterManagerError> {
        self.create_character_image(character, ImageFraming::FullBodyPose)
            .await
    }

    async fn create_character_image(
        &self,
        character: &Character,
        framing: ImageFraming,
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
            .generate_character_image(&prompt, &destination, &character.loras)
            .await?;

        log::info!("generated an image of {} at {destination}", character.name);
        Ok(())
    }

    async fn create_character_image_prompt(
        &self,
        character: &Character,
        kind: ImagePromptKind,
    ) -> Result<Character, CharacterManagerError> {
        if character.has_image_loras()
            && matches!(kind, ImagePromptKind::Body | ImagePromptKind::Outfit)
        {
            return Ok(character.clone());
        }

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
            ImagePromptKind::Body => {
                let messages = fetch_last_n(&mut conn, character.adventure_id, 50).await?;
                ContextBuilder::build_current_body_prompt(
                    &adventure, character, conditions, &messages,
                )?
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

        let mut updated = character.clone();
        set_image_prompt(&mut updated, kind, &generated_prompt);

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        match kind {
            ImagePromptKind::Body => {
                update_appearance_current_body(
                    &mut conn,
                    character.id,
                    updated.appearance_current_body.as_deref(),
                )
                .await?
            }
            ImagePromptKind::Outfit => {
                update_appearance_current_outfit(
                    &mut conn,
                    character.id,
                    &updated.appearance_current_outfit,
                )
                .await?
            }
            ImagePromptKind::Pose => {
                update_appearance_current_pose(
                    &mut conn,
                    character.id,
                    updated
                        .appearance_current_pose
                        .as_deref()
                        .unwrap_or_default(),
                )
                .await?
            }
        }
        drop(conn);

        self.update_image_prompt_shared_state(character.id, kind, &generated_prompt)?;
        log::info!("stored {} prompt for {}", kind.label(), character.name);

        Ok(updated)
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

/// A body prompt that only restates the identity means the story changed nothing, so the
/// slot stays empty and `current_look` answers with the identity instead.
fn body_change(character: &Character, prompt: &str) -> Option<String> {
    let generated = prompt.trim();
    let identity = character.identity().unwrap_or_default().trim();

    (!generated.is_empty() && generated != identity).then(|| generated.to_string())
}

fn set_image_prompt(character: &mut Character, kind: ImagePromptKind, prompt: &str) {
    match kind {
        ImagePromptKind::Body => character.appearance_current_body = body_change(character, prompt),
        ImagePromptKind::Outfit => character.appearance_current_outfit = prompt.to_string(),
        ImagePromptKind::Pose => character.appearance_current_pose = Some(prompt.to_string()),
    }
}

/// A six-character hex stem keeps adapter names short and free of the author's file layout.
fn adapter_name(source: &std::path::Path, dir: &std::path::Path) -> String {
    let extension = source
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();

    for _ in 0..16 {
        let stem = &uuid::Uuid::new_v4().simple().to_string()[..6];
        let name = format!("{stem}{extension}");
        if !dir.join(&name).exists() {
            return name;
        }
    }

    format!("{}{extension}", uuid::Uuid::new_v4().simple())
}
