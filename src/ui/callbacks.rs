use crate::{
    app::AppContext,
    backend::application::port::chat_port::{ChatHandler, NarratorHandler},
    backend::application::port::memory_port::MemoryDistiller,
    ui::RootWindow,
};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn register_callbacks<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
>(
    _ui: &RootWindow,
    _ctx: &AppContext<C, N, M>,
) {
}
