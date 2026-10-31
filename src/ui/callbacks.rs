use slint::ComponentHandle;

use crate::{
    app::AppContext,
    backend::application::port::chat_port::{ChatHandler, NarratorHandler},
    backend::application::port::memory_port::{ActCloser, MemoryDistiller},
    ui::RootWindow,
};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn register_callbacks<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    _ctx: &AppContext<C, N, M, A>,
) {
    let _ui_weak = ui.as_weak();

    ui.on_adventure_selected(move |id| {
        // The id is the uuid string we published in `fill_root_window_inputs`.
        // Slint has already switched to the chat page; here we only react.
        log::info!("adventure selected: {id}");
        // TODO: load the full Adventure and hand it to ctx.adventures /
        //       ctx.messages before the chat page needs it.
    });
}
