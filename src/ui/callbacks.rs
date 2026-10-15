use slint::ComponentHandle;

use crate::backend::application::state::SharedState;
use crate::backend::domain::adventure::Adventure;
use crate::ui::{ChatWindow, MainWindow, window};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn register_callbacks(ui: &MainWindow, state: SharedState) {
    register_main_callbacks(ui, state);
}

// ---------------------------------------------------------------------------
// Home / Adventures page
// ---------------------------------------------------------------------------

fn register_main_callbacks(ui: &MainWindow, state: SharedState) {
    let home_weak = ui.as_weak();

    ui.on_adventure_selected(move |title| {
        if let Err(err) = open_chat_from_home(&home_weak, &state, title.as_str()) {
            log::error!("failed to open chat window: {err}");
        }
    });
}

// ---------------------------------------------------------------------------
// Chat page
// ---------------------------------------------------------------------------

fn register_chat_callbacks(chat: &ChatWindow, home_weak: slint::Weak<MainWindow>) {
    let chat_weak = chat.as_weak();

    chat.on_nav_tab_selected(move |index| {
        if index != 0 {
            return;
        }
        if let Some(chat) = chat_weak.upgrade() {
            let _ = chat.hide();
        }
        if let Some(home) = home_weak.upgrade() {
            let _ = home.show();
        }
    });
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

fn open_chat_from_home(
    home_weak: &slint::Weak<MainWindow>,
    state: &SharedState,
    _adventure_title: &str,
) -> Result<(), slint::PlatformError> {
    let Some(home) = home_weak.upgrade() else {
        return Ok(());
    };

    let adventure = current_adventure(state);
    if adventure.is_none() {
        // TODO: Inform error with a tooltip
        log::warn!("no current adventure in state when opening chat");
    }

    let chat = ChatWindow::new()?;
    window::fill_chat_window_inputs(&chat, adventure.as_ref());
    register_chat_callbacks(&chat, home_weak.clone());

    home.hide()?;
    chat.show()?;
    Ok(())
}

fn current_adventure(state: &SharedState) -> Option<Adventure> {
    state
        .read()
        .ok()
        .and_then(|guard| guard.current_adventure.clone())
}
