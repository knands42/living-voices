use slint::{ModelRc, SharedString, VecModel};

use crate::backend::domain::adventure::Adventure;
use crate::ui::{AdventureSummary as SlintAdventureSummary, RootWindow};

pub fn fill_root_window_inputs(ui: &RootWindow, summaries: &[Adventure]) {
    let items: Vec<SlintAdventureSummary> = summaries
        .iter()
        .map(|a| SlintAdventureSummary {
            id: SharedString::from(a.id.to_string()),
            title: SharedString::from(a.title.clone()),
            description: SharedString::from(a.headline.clone()),
        })
        .collect();

    let model: ModelRc<SlintAdventureSummary> = ModelRc::new(VecModel::from(items));
    ui.set_adventures(model);
}
