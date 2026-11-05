use crate::ui::callbacks::adventure_creation::{
    register_act_creation, register_adventure_opening, register_story_creation,
};
use crate::ui::callbacks::adventure_selected::{
    register_adventure_selected, register_draft_adventure_selected,
};
use crate::ui::callbacks::character_page::register_character_page;
use crate::ui::callbacks::create_adventure::register_create_adventure;
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
    register_draft_adventure_selected(ui, ctx);
    register_message_sent(ui, ctx);
    register_create_adventure(ui, ctx);
    register_story_creation(ui, ctx);
    register_character_page(ui, ctx);
    register_act_creation(ui, ctx);
    register_adventure_opening(ui, ctx);
}
