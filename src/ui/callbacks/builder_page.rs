use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::application::character_gallery::CharacterGallery;
use crate::ui::{BuilderGlobals, RootWindow, viewmodels};
use slint::ComponentHandle;
use tokio::runtime::Handle;
use uuid::Uuid;

/// The builder page's data in, and its events out.
///
/// There is no cache here: the backend snapshot (`AppState`) is the only copy, and
/// every write keeps it current (`load_adventure`, `update_adventure`, `upsert_act`,
/// `create_initial_messages`, `CharacterManager`). Rendering therefore costs no I/O and
/// there is never a second copy to reconcile.
///
/// The rule: **fetch when the page is entered, render from the snapshot everywhere
/// else.** Saves write and then re-render; they never re-read.
#[derive(Clone)]
pub struct BuilderPageContext {
    handle: Handle,
    adventures: AdventureManager,
    gallery: CharacterGallery,
    ui_weak: slint::Weak<RootWindow>,
}

impl BuilderPageContext {
    pub fn new(
        handle: Handle,
        adventures: AdventureManager,
        gallery: CharacterGallery,
        ui_weak: slint::Weak<RootWindow>,
    ) -> Self {
        BuilderPageContext {
            handle,
            adventures,
            gallery,
            ui_weak,
        }
    }

    /// The one place that reads the database for the builder: it warms the snapshot
    /// with the adventure, its acts and whether the opening exists.
    pub fn open(&self, adventure_id: Uuid) -> Result<(), AdventureManagerError> {
        tokio::task::block_in_place(|| {
            self.handle
                .block_on(self.adventures.load_adventure(adventure_id))
        })?;

        self.show(Some("story"));
        self.reveal();

        Ok(())
    }

    /// Pushes the builder view into Slint: the adventure comes from the snapshot, and
    /// the two lists the view shows are read here, as small indexed queries.
    pub fn show(&self, section: Option<&str>) {
        let Some(ui) = self.ui_weak.upgrade() else {
            return;
        };
        let Some(adventure) = self.adventures.current_adventure() else {
            log::error!("the builder view was requested before an adventure was loaded");
            return;
        };

        let lists = tokio::task::block_in_place(|| {
            self.handle.block_on(async {
                let acts = self.adventures.get_acts(adventure.id).await?;
                let opening = self.adventures.get_messages(adventure.id, 10).await?;
                Ok::<_, AdventureManagerError>((acts, !opening.is_empty()))
            })
        });

        let (acts, has_opening) = match lists {
            Ok(lists) => lists,
            Err(err) => {
                log::error!("could not read the lists of {}: {err}", adventure.id);
                return;
            }
        };

        let builder = ui.global::<BuilderGlobals>();
        builder.set_builder(viewmodels::builder_view(
            Some(&adventure),
            &acts,
            has_opening,
            &self.gallery,
        ));

        if let Some(section) = section {
            builder.set_section(section.into());
        }
    }

    /// The builder page became visible again: render it and switch to it.
    pub fn enter(&self) {
        let section = self
            .ui_weak
            .upgrade()
            .map(|ui| ui.global::<BuilderGlobals>().get_section().to_string())
            .filter(|section| !section.is_empty())
            .unwrap_or_else(|| "story".to_string());

        self.show(Some(section.as_str()));
        self.reveal();
    }

    fn reveal(&self) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_current_page("builder".into());
        }
    }
}

pub fn register_builder_page(ui: &RootWindow, builder: &BuilderPageContext) {
    let section_page = builder.clone();
    ui.global::<BuilderGlobals>()
        .on_section_requested(move |section| {
            section_page.show(Some(section.as_str()));
        });

    let entered_page = builder.clone();
    ui.global::<BuilderGlobals>()
        .on_builder_entered(move || entered_page.enter());
}
