use crate::app::AppContext;
use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::application::port::chat_port::{ChatHandler, NarratorHandler};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::Act;
use crate::backend::domain::message::Message;
use crate::ui::callbacks::builder_page::BuilderPageContext;
use crate::ui::{ActInputView, BuilderGlobals, RootWindow, StoryInputView};
use slint::ComponentHandle;
use thiserror::Error;
use tokio::runtime::Handle;
use uuid::Uuid;

#[derive(Error, Debug)]
enum CreationError {
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Adventure(#[from] AdventureManagerError),
}

pub fn register_story_creation<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
    builder: &BuilderPageContext,
) {
    let adventures = ctx.adventures.clone();
    let handle = Handle::current();
    let builder = builder.clone();

    ui.global::<BuilderGlobals>()
        .on_save_story(move |adventure_id, input| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            let result = tokio::task::block_in_place(|| {
                handle.block_on(save_story(&adventures, adventure_id, input))
            });

            match result {
                Ok(()) => builder.show(None),
                Err(err) => log::error!("could not save the story of {adventure_id}: {err}"),
            }
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
    builder: &BuilderPageContext,
) {
    let adventures = ctx.adventures.clone();
    let handle = Handle::current();
    let builder = builder.clone();

    ui.global::<BuilderGlobals>()
        .on_save_act(move |adventure_id, input| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            let acts = builder.acts();
            let result = tokio::task::block_in_place(|| {
                handle.block_on(save_act(&adventures, adventure_id, input, acts))
            });

            match result {
                Ok(()) => {
                    builder.reload_acts();
                    builder.show(None);
                }
                Err(err) => log::error!("could not save an act of {adventure_id}: {err}"),
            }
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
    builder: &BuilderPageContext,
) {
    let adventures = ctx.adventures.clone();
    let handle = Handle::current();
    let builder = builder.clone();

    ui.global::<BuilderGlobals>()
        .on_save_opening(move |adventure_id, text| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            let result = tokio::task::block_in_place(|| {
                handle.block_on(save_opening(&adventures, adventure_id, text.as_str()))
            });

            match result {
                Ok(()) => {
                    builder.mark_opening_written();
                    builder.show(None);
                }
                Err(err) => log::error!("could not save the opening of {adventure_id}: {err}"),
            }
        });
}

/// The story fields live on the adventure, which the snapshot already holds, so this
/// reads memory and writes once. The write refreshes the snapshot in turn.
async fn save_story(
    adventures: &AdventureManager,
    adventure_id: Uuid,
    input: StoryInputView,
) -> Result<(), CreationError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(CreationError::Invalid("title"));
    }

    let mut adventure = current_adventure(adventures, adventure_id)?;
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

async fn save_act(
    adventures: &AdventureManager,
    adventure_id: Uuid,
    input: ActInputView,
    acts: Vec<Act>,
) -> Result<(), CreationError> {
    let goal = input.goal.trim();
    if goal.is_empty() {
        return Err(CreationError::Invalid("act goal"));
    }
    let trigger = Some(input.trigger.trim())
        .filter(|trigger| !trigger.is_empty())
        .map(str::to_string);

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
    adventures: &AdventureManager,
    adventure_id: Uuid,
    text: &str,
) -> Result<(), CreationError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(CreationError::Invalid("opening"));
    }

    let adventure = current_adventure(adventures, adventure_id)?;
    let message = adventure.message_from_reply(Message::NARRATOR_SPEAKER_NAME, text);
    adventures.create_initial_messages(&[message]).await?;
    Ok(())
}

/// The adventure the page edits, read from the snapshot rather than the database.
fn current_adventure(
    adventures: &AdventureManager,
    adventure_id: Uuid,
) -> Result<Adventure, CreationError> {
    adventures
        .current_adventure()
        .filter(|adventure| adventure.id == adventure_id)
        .ok_or(CreationError::Invalid("adventure"))
}

pub fn parse_id(id: &str) -> Option<Uuid> {
    Uuid::parse_str(id)
        .map_err(|_| log::error!("invalid id: {id}"))
        .ok()
}
