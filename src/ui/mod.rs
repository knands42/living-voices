use crate::app::UiContext;

mod callbacks;
pub mod viewmodels;
mod window;

slint::include_modules!();

pub fn run(ctx: UiContext) -> Result<(), slint::PlatformError> {
    let ui = RootWindow::new()?;

    let session = ctx.adventures.current_adventure();
    window::fill_root_window_inputs(&ui, session.as_ref());
    callbacks::register_callbacks(&ui, &ctx);

    ui.run()
}
