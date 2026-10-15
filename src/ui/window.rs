use crate::{backend::domain::adventure::Adventure, ui::RootWindow};

/// Populate the root window with data from the current session.
/// Both pages read from the same window, so this is the one place to wire
/// backend data in.
pub fn fill_root_window_inputs(_ui: &RootWindow, _session: Option<&Adventure>) {}
