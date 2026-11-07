use crate::ui::callbacks::adventure_creation::{
    register_act_creation, register_adventure_opening, register_create_adventure,
    register_story_creation,
};
use crate::ui::callbacks::adventure_selected::{
    register_adventure_selected, register_draft_adventure_selected,
};
use crate::ui::callbacks::builder_page::{BuilderPageContext, register_builder_page};
use crate::ui::callbacks::character_page::register_character_page;
use crate::ui::callbacks::message_send::register_message_sent;
use crate::{app::DefaultContext, ui::RootWindow};
use slint::ComponentHandle;
use tokio::runtime::Handle;

pub fn register_callbacks(ui: &RootWindow, ctx: &DefaultContext) {
    // One cache for the whole app: the builder fetches it when its page opens and
    // The builder needs nothing cached: the backend snapshot is the only copy, and
    // this context is just how the page is opened and rendered.
    let builder = BuilderPageContext::new(
        Handle::current(),
        ctx.adventures.clone(),
        ctx.characters.gallery().clone(),
        ui.as_weak(),
    );

    register_adventure_selected(ui, ctx);
    register_draft_adventure_selected(ui, &builder);
    register_builder_page(ui, &builder);
    register_message_sent(ui, ctx);
    register_create_adventure(ui, ctx);
    register_story_creation(ui, ctx, &builder);
    register_character_page(ui, ctx);
    register_act_creation(ui, ctx, &builder);
    register_adventure_opening(ui, ctx, &builder);
}
