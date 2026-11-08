use crate::app::DefaultContext;
use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::domain::adventure::Adventure;
use crate::ui::{AdventureSummaryView, MainGlobals, RootWindow};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use thiserror::Error;
use tokio::runtime::Handle;

const DEFAULT_TONE: &str = "neutral";

#[derive(Error, Debug)]
enum MainPageError {
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Adventure(#[from] AdventureManagerError),
}

/// Everything a main-page callback needs, cheap to clone into each closure.
///
/// The page keeps no snapshot of its own: a draft is created and its card appended
/// straight to the list the player is looking at.
#[derive(Clone)]
struct MainPageContext {
    handle: Handle,
    adventures: AdventureManager,
    ui_weak: slint::Weak<RootWindow>,
}

impl MainPageContext {
    fn create_draft(&self, title: &str, headline: &str) -> Result<Adventure, MainPageError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(MainPageError::Invalid("title"));
        }

        tokio::task::block_in_place(|| {
            self.handle.block_on(self.adventures.create_adventure(
                title.to_string(),
                headline.trim().to_string(),
                DEFAULT_TONE.to_string(),
            ))
        })
        .map_err(MainPageError::from)
    }

    /// Callbacks run on the UI thread, so the list can be updated directly.
    fn append_summary(&self, adventure: &Adventure) {
        let Some(ui) = self.ui_weak.upgrade() else {
            return;
        };

        let summary = AdventureSummaryView {
            id: SharedString::from(adventure.id.to_string()),
            title: SharedString::from(adventure.title.clone()),
            description: SharedString::from(adventure.headline.clone()),
            status: SharedString::from(adventure.status.as_str()),
        };

        let main = ui.global::<MainGlobals>();
        let model = main.get_adventures_summary();
        match model
            .as_any()
            .downcast_ref::<VecModel<AdventureSummaryView>>()
        {
            Some(vec_model) => vec_model.push(summary),
            None => {
                let mut items: Vec<AdventureSummaryView> = model.iter().collect();
                items.push(summary);
                main.set_adventures_summary(ModelRc::new(VecModel::from(items)));
            }
        }
    }
}

pub fn register_main_page(ui: &RootWindow, ctx: &DefaultContext) {
    let env = MainPageContext {
        handle: Handle::current(),
        adventures: ctx.adventures.clone(),
        ui_weak: ui.as_weak(),
    };

    register_create_adventure(ui, &env);
}

/// Main page: the new-adventure form returns the draft's id so the page can select it.
fn register_create_adventure(ui: &RootWindow, env: &MainPageContext) {
    let env = env.clone();
    ui.global::<MainGlobals>()
        .on_create_adventure(move |title, headline| {
            let adventure = match env.create_draft(title.as_str(), headline.as_str()) {
                Ok(adventure) => adventure,
                Err(err) => {
                    log::error!("could not create a draft adventure: {err}");
                    return SharedString::new();
                }
            };

            log::info!("draft adventure created: {}", adventure.id);
            env.append_summary(&adventure);
            SharedString::from(adventure.id.to_string())
        });
}
