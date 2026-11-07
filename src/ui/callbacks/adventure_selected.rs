use crate::app::DefaultContext;
use crate::backend::application::adventure_manager::{AdventureManagerError, MemorySnapshot};
use crate::backend::application::character_gallery::CharacterGallery;
use crate::backend::domain::message::Message;
use crate::ui::callbacks::builder_page::BuilderPageContext;
use crate::ui::{ChatGlobals, MainGlobals, RootWindow, viewmodels};
use slint::ComponentHandle;
use tokio::runtime::Handle;
use uuid::Uuid;

pub fn register_adventure_selected(ui: &RootWindow, ctx: &DefaultContext) {
    let ctx = ctx.clone();
    let handle = Handle::current();
    let ui_weak = ui.as_weak();

    ui.global::<MainGlobals>().on_adventure_selected(move |id| {
        let Ok(adventure_id) = Uuid::parse_str(id.as_str()) else {
            log::error!("invalid adventure id: {id}");
            return;
        };

        let (snapshot, messages) = match load_and_start(&handle, &ctx, adventure_id) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                log::error!("could not open adventure {adventure_id}: {err}");
                // TODO: Create a notification tooltip-like and return to main page
                return;
            }
        };

        apply_snapshot(
            &ui_weak,
            snapshot,
            messages,
            ctx.characters.gallery().clone(),
        );
    });
}

/// The builder owns the load, so this needs no context of its own.
pub fn register_draft_adventure_selected(ui: &RootWindow, builder: &BuilderPageContext) {
    let builder = builder.clone();

    ui.global::<MainGlobals>()
        .on_draft_adventure_selected(move |id| {
            let Ok(adventure_id) = Uuid::parse_str(id.as_str()) else {
                log::error!("invalid adventure id: {id}");
                return;
            };

            let result = builder.open(adventure_id);

            if let Err(err) = result {
                log::error!("could not open draft {adventure_id}: {err}");
                // TODO: Create a notification tooltip-like and return to main page
            }
        });
}

fn load_and_start(
    handle: &Handle,
    ctx: &DefaultContext,
    adventure_id: Uuid,
) -> Result<(MemorySnapshot, Vec<Message>), AdventureManagerError> {
    tokio::task::block_in_place(|| {
        handle.block_on(async {
            let adventure = ctx.adventures.get_adventure(adventure_id).await?;
            ctx.adventures.start_adventure(adventure.id).await?;
            let messages = ctx.adventures.get_messages(adventure.id, 50).await?;
            log::info!("adventure started: {adventure_id}");

            Ok((ctx.adventures.current_snapshot(), messages))
        })
    })
}

fn apply_snapshot(
    ui_weak: &slint::Weak<RootWindow>,
    snapshot: MemorySnapshot,
    messages: Vec<Message>,
    gallery: CharacterGallery,
) {
    let ui_weak = ui_weak.clone();
    let post = slint::invoke_from_event_loop(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        if let Some(player) = snapshot.player {
            let chat = ui.global::<ChatGlobals>();
            chat.set_adventure(viewmodels::adventure_view(
                snapshot.current_adventure.as_ref(),
                &gallery,
            ));
            chat.set_act(viewmodels::act_view(snapshot.current_act.as_ref()));
            chat.set_state(viewmodels::state_view(snapshot.current_state.as_ref()));
            chat.set_facts(viewmodels::facts_model(&snapshot.current_facts));
            chat.set_messages(viewmodels::messages_model(&messages, Some(player.id)));
            ui.set_current_page("chat".into());
        };
    });

    if let Err(err) = post {
        log::error!("could not post adventure to UI: {err}");
    }
}
