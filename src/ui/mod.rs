use crate::app::AppContext;
use crate::backend::application::port::chat_port::ChatHandler;
use crate::backend::application::port::memory_port::MemoryDistiller;

mod callbacks;
pub mod viewmodels;
mod window;

slint::include_modules!();

pub fn run<C: ChatHandler + Clone + 'static, M: MemoryDistiller + Clone + 'static>(
    ctx: AppContext<C, M>,
) -> Result<(), slint::PlatformError> {
    let ui = RootWindow::new()?;

    let session = ctx.adventures.current_adventure();
    window::fill_root_window_inputs(&ui, session.as_ref());
    callbacks::register_callbacks(&ui, &ctx);

    ui.run()
}
