use crate::app::AppContext;
use crate::backend::application::adventure_manager::{AdventureManagerError, MemorySnapshot};
use crate::backend::application::port::chat_port::{ChatHandler, NarratorHandler};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::domain::message::Message;
use crate::ui::{RootWindow, viewmodels};
use slint::ComponentHandle;
use tokio::runtime::Handle;
use uuid::Uuid;

pub fn register_adventure_selected<
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

    ui.on_adventure_selected(move |id| {
        let Ok(adventure_id) = Uuid::parse_str(id.as_str()) else {
            log::error!("invalid adventure id: {id}");
            return;
        };

        let (snapshot, messages) = match load_and_start(&handle, &ctx, adventure_id) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                log::error!("could not open adventure {adventure_id}: {err}");
                return;
            }
        };

        apply_snapshot(&ui_weak, snapshot, messages);
    });
}

fn load_and_start<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    handle: &Handle,
    ctx: &AppContext<C, N, M, A>,
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
) {
    let ui_weak = ui_weak.clone();
    let post = slint::invoke_from_event_loop(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        if let Some(player) = snapshot.player {
            ui.set_current_adventure(viewmodels::adventure_view(
                snapshot.current_adventure.as_ref(),
            ));
            ui.set_current_act(viewmodels::act_view(snapshot.current_act.as_ref()));
            ui.set_current_state(viewmodels::state_view(snapshot.current_state.as_ref()));
            ui.set_current_facts(viewmodels::facts_model(&snapshot.current_facts));
            ui.set_current_messages(viewmodels::messages_model(&messages, Some(player.id)));
        };
    });

    if let Err(err) = post {
        log::error!("could not post adventure to UI: {err}");
    }
}
