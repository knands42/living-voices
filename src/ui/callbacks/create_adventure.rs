use crate::app::AppContext;
use crate::backend::application::adventure_manager::AdventureManagerError;
use crate::backend::application::port::chat_port::{ChatHandler, NarratorHandler};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::domain::adventure::Adventure;
use crate::ui::{AdventureSummary, RootWindow};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use tokio::runtime::Handle;

const DEFAULT_TONE: &str = "neutral";

pub fn register_create_adventure<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
) {
    let ctx = ctx.clone();
    let handle = Handle::current();
    let ui_weak = ui.as_weak();

    ui.on_create_adventure(move |title, headline| {
        if title.trim().is_empty() {
            log::error!("create adventure called without a title");
            return;
        }

        let adventure = match create_draft(&handle, &ctx, title.as_str(), headline.as_str()) {
            Ok(adventure) => adventure,
            Err(err) => {
                log::error!("could not create draft adventure: {err}");
                return;
            }
        };

        log::info!("draft adventure created: {}", adventure.id);
        append_summary(&ui_weak, &adventure);
    });
}

fn create_draft<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    handle: &Handle,
    ctx: &AppContext<C, N, M, A>,
    title: &str,
    headline: &str,
) -> Result<Adventure, AdventureManagerError> {
    tokio::task::block_in_place(|| {
        handle.block_on(ctx.adventures.create_adventure(
            title.to_string(),
            headline.to_string(),
            DEFAULT_TONE.to_string(),
        ))
    })
}

fn append_summary(ui_weak: &slint::Weak<RootWindow>, adventure: &Adventure) {
    let summary = AdventureSummary {
        id: SharedString::from(adventure.id.to_string()),
        title: SharedString::from(adventure.title.clone()),
        description: SharedString::from(adventure.headline.clone()),
    };

    let ui_weak = ui_weak.clone();
    let post = slint::invoke_from_event_loop(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        let model = ui.get_adventures();
        if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<AdventureSummary>>() {
            vec_model.push(summary);
        } else {
            let mut items: Vec<AdventureSummary> = model.iter().collect();
            items.push(summary);
            ui.set_adventures(ModelRc::new(VecModel::from(items)));
        }
    });

    if let Err(err) = post {
        log::error!("could not add draft to the adventure list: {err}");
    }
}
