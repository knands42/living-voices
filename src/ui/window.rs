use crate::backend::domain::adventure::AdventureSummary;
use crate::ui::{RootWindow, viewmodels};

pub fn fill_root_window_inputs(ui: &RootWindow, summaries: &[AdventureSummary]) {
    ui.set_adventures_summary(viewmodels::adventure_summaries_model(summaries));
}
