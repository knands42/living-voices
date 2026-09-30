use crate::backend::application::state::SharedState;

mod callbacks;
pub mod viewmodels;
mod window;

slint::include_modules!();

pub fn run(state: SharedState) -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;

    let session = state
        .read()
        .ok()
        .and_then(|guard| guard.current_session.clone());
    window::fill_main_window_inputs(&ui, session.as_ref());
    callbacks::register_callbacks(&ui, state);

    ui.run()
}
