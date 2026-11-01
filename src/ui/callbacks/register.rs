use crate::ui::callbacks::adventure_selected::register_adventure_selected;
use crate::ui::callbacks::message_send::register_message_sent;
use crate::{
    app::AppContext,
    backend::application::port::chat_port::{ChatHandler, NarratorHandler},
    backend::application::port::memory_port::{ActCloser, MemoryDistiller},
    ui::RootWindow,
};

pub fn register_callbacks<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
) {
    register_adventure_selected(ui, ctx);
    register_message_sent(ui, ctx);
}
