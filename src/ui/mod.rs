use crate::app::AppContext;
use crate::backend::application::port::chat_port::ChatHandler;

mod callbacks;
pub mod viewmodels;
mod window;

slint::include_modules!();

pub fn run<C: ChatHandler + Clone + 'static>(
    ctx: AppContext<C>,
) -> Result<(), slint::PlatformError> {
    let ui = RootWindow::new()?;

    let session = ctx.adventures.current_adventure();
    window::fill_root_window_inputs(&ui, session.as_ref());
    callbacks::register_callbacks(&ui, &ctx);

    ui.run()
}
