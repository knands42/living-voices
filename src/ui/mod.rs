use crate::app::AppContext;
use crate::backend::application::port::chat_port::{ChatHandler, NarratorHandler};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};

mod callbacks;
pub mod viewmodels;
mod window;

slint::include_modules!();

pub fn run<
    CHAT: ChatHandler + Clone + 'static,
    NARRATOR: NarratorHandler + Clone + 'static,
    MEMORY: MemoryDistiller + Clone + 'static,
    ACT: ActCloser + Clone + 'static,
>(
    ctx: AppContext<CHAT, NARRATOR, MEMORY, ACT>,
) -> Result<(), slint::PlatformError> {
    let ui = RootWindow::new()?;

    let session = ctx.adventures.current_adventure();
    window::fill_root_window_inputs(&ui, session.as_ref());
    callbacks::register_callbacks(&ui, &ctx);

    ui.run()
}
