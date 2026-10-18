use crate::{backend::domain::adventure::Adventure, ui::RootWindow};

/// The one place to wire backend data into the root window, read by both pages.
pub fn fill_root_window_inputs(_ui: &RootWindow, _session: Option<&Adventure>) {}
