use crate::ui::callbacks::adventure_creation::{
    register_act_creation, register_adventure_opening, register_story_creation,
};
use crate::ui::callbacks::adventure_selected::{
    register_adventure_selected, register_draft_adventure_selected,
};
use crate::ui::callbacks::builder_page::{BuilderPageContext, register_builder_page};
use crate::ui::callbacks::character_page::register_character_page;
use crate::ui::callbacks::create_adventure::register_create_adventure;
use crate::ui::callbacks::message_send::register_message_sent;
use crate::{
    app::AppContext,
    backend::application::port::chat_port::{ChatHandler, NarratorHandler},
    backend::application::port::memory_port::{ActCloser, MemoryDistiller},
    ui::RootWindow,
};
use slint::ComponentHandle;
use tokio::runtime::Handle;

pub fn register_callbacks<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
) {
    // One cache for the whole app: the builder fetches it when its page opens and
    // every other callback renders from it instead of re-reading.
    let builder = BuilderPageContext::new(Handle::current(), ctx.adventures.clone(), ui.as_weak());

    register_adventure_selected(ui, ctx);
    register_draft_adventure_selected(ui, &builder);
    register_builder_page(ui, &builder);
    register_message_sent(ui, ctx);
    register_create_adventure(ui, ctx);
    register_story_creation(ui, ctx, &builder);
    register_character_page(ui, ctx, &builder);
    register_act_creation(ui, ctx, &builder);
    register_adventure_opening(ui, ctx, &builder);
}
