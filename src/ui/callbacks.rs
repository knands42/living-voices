use slint::ComponentHandle;

use crate::backend::application::state::SharedState;
use crate::backend::domain::adventure::Adventure;
use crate::ui::RootWindow;

const PAGE_HOME: i32 = 0;
const PAGE_CHAT: i32 = 1;
const TAB_CAMPAIGNS: i32 = 0;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn register_callbacks(ui: &RootWindow, state: SharedState) {
    register_main_callbacks(ui, state.clone());
    register_chat_callbacks(ui, state);
    register_shared_callbacks(ui);
}

// ---------------------------------------------------------------------------
// Home / Adventures page
// ---------------------------------------------------------------------------

fn register_main_callbacks(ui: &RootWindow, _state: SharedState) {
    let ui_weak = ui.as_weak();

    ui.on_adventure_selected(move |title| {
        log::info!("adventure selected: {title}");
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_page(PAGE_CHAT);
            ui.set_active_tab(TAB_CAMPAIGNS);
        }
    });
}

// ---------------------------------------------------------------------------
// Chat page
// ---------------------------------------------------------------------------

fn register_chat_callbacks(ui: &RootWindow, state: SharedState) {
    
}

// ---------------------------------------------------------------------------
// Shared chrome (top bar nav)
// ---------------------------------------------------------------------------

fn register_shared_callbacks(ui: &RootWindow) {
    let ui_weak = ui.as_weak();

    ui.on_nav_tab_selected(move |index| {
        let Some(ui) = ui_weak.upgrade() else { return };
        if index == TAB_CAMPAIGNS {
            ui.set_page(PAGE_HOME);
        }
    });
}

fn current_adventure(state: &SharedState) -> Option<Adventure> {
    state
        .read()
        .ok()
        .and_then(|guard| guard.current_adventure.clone())
}