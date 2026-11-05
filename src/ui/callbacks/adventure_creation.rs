use crate::ai::integrations::text::chat::Chat;
use crate::app::AppContext;
use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::application::character_manager::{CharacterManager, CharacterManagerError};
use crate::backend::application::port::chat_port::{ChatHandler, NarratorHandler};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::{Character, CharacterStatus};
use crate::backend::domain::memory::Act;
use crate::backend::domain::message::Message;
use crate::ui::{
    ActInputView, BuilderGlobals, CharacterInputView, RootWindow, StoryInputView, viewmodels,
};
use slint::ComponentHandle;
use std::future::Future;
use thiserror::Error;
use tokio::runtime::Handle;
use uuid::Uuid;

#[derive(Error, Debug)]
enum CreationError {
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Adventure(#[from] AdventureManagerError),
    #[error(transparent)]
    Character(#[from] CharacterManagerError),
}

pub struct BuilderData {
    adventure: Adventure,
    acts: Vec<Act>,
    has_opening: bool,
}

pub async fn load_builder_data(
    adventure_manager: &AdventureManager,
    adventure_id: Uuid,
) -> Result<BuilderData, AdventureManagerError> {
    Ok(BuilderData {
        adventure: adventure_manager.get_adventure(adventure_id).await?,
        acts: adventure_manager.get_acts(adventure_id).await?,
        has_opening: !adventure_manager
            .get_messages(adventure_id, 1)
            .await?
            .is_empty(),
    })
}

// Callbacks run on the UI thread, so the window can be updated directly.
pub fn show_builder(ui_weak: &slint::Weak<RootWindow>, data: &BuilderData, open_page: bool) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };

    ui.global::<BuilderGlobals>()
        .set_builder(viewmodels::builder_view(
            Some(&data.adventure),
            &data.acts,
            data.has_opening,
        ));
    if open_page {
        ui.set_current_page("builder".into());
    }
}

pub fn register_story_creation<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
) {
    let ctx = ctx.clone();
    let handle = Handle::current();
    let ui_weak = ui.as_weak();

    ui.global::<BuilderGlobals>()
        .on_save_story(move |adventure_id, input| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            save_and_refresh(
                &handle,
                &ctx.adventures,
                &ui_weak,
                adventure_id,
                "save story",
                save_story(&ctx.adventures, adventure_id, input),
            );
        });
}

pub fn register_cast_creation<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
) {
    let ctx = ctx.clone();
    let handle = Handle::current();
    let ui_weak = ui.as_weak();

    ui.global::<BuilderGlobals>()
        .on_save_character(move |adventure_id, input| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            save_and_refresh(
                &handle,
                &ctx.adventures,
                &ui_weak,
                adventure_id,
                "save character",
                save_character(&ctx.adventures, &ctx.characters, adventure_id, input),
            );
        });
}

pub fn register_act_creation<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
) {
    let ctx = ctx.clone();
    let handle = Handle::current();
    let ui_weak = ui.as_weak();

    ui.global::<BuilderGlobals>()
        .on_save_act(move |adventure_id, input| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            save_and_refresh(
                &handle,
                &ctx.adventures,
                &ui_weak,
                adventure_id,
                "save act",
                save_act(&ctx.adventures, adventure_id, input),
            );
        });
}

pub fn register_adventure_opening<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
) {
    let ctx = ctx.clone();
    let handle = Handle::current();
    let ui_weak = ui.as_weak();

    ui.global::<BuilderGlobals>()
        .on_save_opening(move |adventure_id, text| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            save_and_refresh(
                &handle,
                &ctx.adventures,
                &ui_weak,
                adventure_id,
                "save opening",
                save_opening(&ctx.adventures, adventure_id, text.as_str()),
            );
        });
}

/// Saves pending changes, reloads the adventure builder data, and refreshes the UI.
fn save_and_refresh(
    handle: &Handle,
    adventures: &AdventureManager,
    ui_weak: &slint::Weak<RootWindow>,
    adventure_id: Uuid,
    action: &str,
    save: impl Future<Output = Result<(), CreationError>>,
) {
    let result = tokio::task::block_in_place(|| {
        handle.block_on(async {
            save.await?;
            Ok::<_, CreationError>(load_builder_data(adventures, adventure_id).await?)
        })
    });

    match result {
        Ok(data) => show_builder(ui_weak, &data, false),
        Err(err) => log::error!("could not {action} for adventure {adventure_id}: {err}"),
    }
}

async fn save_story(
    adventures: &AdventureManager,
    adventure_id: Uuid,
    input: StoryInputView,
) -> Result<(), CreationError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(CreationError::Invalid("title"));
    }

    let mut adventure = adventures.get_adventure(adventure_id).await?;
    adventure.title = title.to_string();
    adventure.headline = input.headline.trim().to_string();
    adventure.tone = input.tone.trim().to_string();
    adventure.lore.text = input.lore.trim().to_string();
    if !input.narrator_name.trim().is_empty() {
        adventure.narrator.name = input.narrator_name.trim().to_string();
    }

    adventures.update_adventure(&adventure).await?;
    Ok(())
}

async fn save_character(
    adventures: &AdventureManager,
    characters: &CharacterManager<Chat>,
    adventure_id: Uuid,
    input: CharacterInputView,
) -> Result<(), CreationError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(CreationError::Invalid("character name"));
    }

    if input.id.is_empty() {
        let character = Character::new(
            Uuid::new_v4(),
            name.to_string(),
            adventure_id,
            input.headline.trim(),
            input.backstory.trim(),
            CharacterStatus::Present,
            false,
        );
        characters.create_character(character).await?;
        return Ok(());
    }

    let character_id = parse_id(input.id.as_str()).ok_or(CreationError::Invalid("character id"))?;
    let mut character = adventures
        .get_adventure(adventure_id)
        .await?
        .characters
        .into_iter()
        .find(|character| character.id == character_id)
        .ok_or(CreationError::Invalid("character id"))?;
    character.name = name.to_string();
    character.headline = input.headline.trim().to_string();
    character.backstory = input.backstory.trim().to_string();

    characters.update_character(character).await?;
    Ok(())
}

async fn save_act(
    adventures: &AdventureManager,
    adventure_id: Uuid,
    input: ActInputView,
) -> Result<(), CreationError> {
    let goal = input.goal.trim();
    if goal.is_empty() {
        return Err(CreationError::Invalid("act goal"));
    }
    let trigger = Some(input.trigger.trim())
        .filter(|trigger| !trigger.is_empty())
        .map(str::to_string);

    let acts = adventures.get_acts(adventure_id).await?;
    let mut act = if input.id.is_empty() {
        let ordinal = acts.iter().map(|act| act.ordinal + 1).max().unwrap_or(0);
        Act::new(adventure_id, ordinal, goal)
    } else {
        let act_id = parse_id(input.id.as_str()).ok_or(CreationError::Invalid("act id"))?;
        acts.into_iter()
            .find(|act| act.id == act_id)
            .ok_or(CreationError::Invalid("act id"))?
    };
    act.goal = goal.to_string();
    act.trigger = trigger;

    adventures.upsert_act(&act).await?;
    Ok(())
}

async fn save_opening(
    adventure_manager: &AdventureManager,
    adventure_id: Uuid,
    text: &str,
) -> Result<(), CreationError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(CreationError::Invalid("opening"));
    }

    let adventure = adventure_manager.get_adventure(adventure_id).await?;
    let message = adventure.message_from_reply(Message::NARRATOR_SPEAKER_NAME, text);
    adventure_manager
        .create_initial_messages(&[message])
        .await?;
    Ok(())
}

fn parse_id(id: &str) -> Option<Uuid> {
    Uuid::parse_str(id)
        .map_err(|_| log::error!("invalid id: {id}"))
        .ok()
}
