use crate::{
    backend::domain::adventure::Adventure,
    ui::{ChatWindow, MainWindow},
};

/// Populate the home window with data from the current session.
/// Empty for now — the Slint defaults mirror the mockup until real data lands.
pub fn fill_main_window_inputs(_ui: &MainWindow, _session: Option<&Adventure>) {}

/// Populate the chat window. Slint-side defaults already match the mockup,
/// so this is a hook for later wiring (characters, messages, clock, ...).
pub fn fill_chat_window_inputs(_ui: &ChatWindow, _session: Option<&Adventure>) {}
