use crate::backend::application::state::SharedState;

mod callbacks;
pub mod viewmodels;
mod window;

slint::include_modules!();

pub fn run(state: SharedState) -> Result<(), slint::PlatformError> {
    let ui = RootWindow::new()?;

    let session = state
        .read()
        .ok()
        .and_then(|guard| guard.current_adventure.clone());
    window::fill_root_window_inputs(&ui, session.as_ref());
    callbacks::register_callbacks(&ui, state);

    ui.run()
}
