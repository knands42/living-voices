use slint::ComponentHandle;

use crate::backend::domain::adventure::AdventureSummary;
use crate::ui::{MainVm, RootWindow, viewmodels};

pub fn fill_root_window_inputs(ui: &RootWindow, summaries: &[AdventureSummary]) {
    ui.global::<MainVm>()
        .set_adventures_summary(viewmodels::adventure_summaries_model(summaries));
}
