use crate::ui::callbacks::builder_page::register_builder_page;
use crate::ui::callbacks::character_page::register_character_page;
use crate::ui::callbacks::chat_page::register_chat_page;
use crate::ui::callbacks::main_page::register_main_page;
use crate::{app::DefaultContext, ui::RootWindow};

pub fn register_callbacks(ui: &RootWindow, ctx: &DefaultContext) {
    register_main_page(ui, ctx);
    register_builder_page(ui, ctx);
    register_character_page(ui, ctx);
    register_chat_page(ui, ctx);
}
