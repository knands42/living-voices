use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::Act;
use crate::ui::{BuilderGlobals, RootWindow, viewmodels};
use slint::ComponentHandle;
use tokio::runtime::Handle;
use uuid::Uuid;

/// Everything the builder holds that the backend snapshot does not: the acts and
/// whether an opening exists.
struct BuilderExtras {
    adventure_id: Uuid,
    acts: Vec<Act>,
    has_opening: bool,
}

/// The builder page's data in, and its only cache.
///
/// The adventure itself is deliberately not stored here. `SharedState.current_adventure`
/// is already kept current by every write that touches it (`update_adventure`,
/// `update_character`, `create_character`), so reading it costs no I/O and there is
/// never a stale second copy to reconcile.
///
/// The rule this exists to enforce: **fetch when the page is entered, render from
/// memory everywhere else.** Saves write and patch the snapshot; they do not re-read.
#[derive(Clone)]
pub struct BuilderPageContext {
    handle: Handle,
    adventures: AdventureManager,
    ui_weak: slint::Weak<RootWindow>,
    extras: Rc<RefCell<Option<BuilderExtras>>>,
}

impl BuilderPageContext {
    pub fn new(
        handle: Handle,
        adventures: AdventureManager,
        ui_weak: slint::Weak<RootWindow>,
    ) -> Self {
        BuilderPageContext {
            handle,
            adventures,
            ui_weak,
            extras: Rc::new(RefCell::new(None)),
        }
    }

    /// The one place that reads the database for the builder. Call it when the page
    /// is opened, never from a save path.
    pub fn open(&self, adventure_id: Uuid) -> Result<(), AdventureManagerError> {
        let extras = tokio::task::block_in_place(|| {
            self.handle.block_on(async {
                // Warms the snapshot as a side effect, which is what `show` renders from.
                self.adventures.load_adventure(adventure_id).await?;
                let acts = self.adventures.get_acts(adventure_id).await?;
                let has_opening = !self
                    .adventures
                    .get_messages(adventure_id, 1)
                    .await?
                    .is_empty();

                Ok::<_, AdventureManagerError>(BuilderExtras {
                    adventure_id,
                    acts,
                    has_opening,
                })
            })
        })?;

        *self.extras.borrow_mut() = Some(extras);
        self.show(Some("story"));
        self.reveal();

        Ok(())
    }

    /// Pushes the builder view into Slint from the snapshot and the cached extras.
    /// No I/O, so it is safe to call after every write and on every tab switch.
    pub fn show(&self, section: Option<&str>) {
        let Some(ui) = self.ui_weak.upgrade() else {
            return;
        };

        // Copy what the view needs out of the cache first: nothing here holds a
        // `RefCell` borrow while Slint is being touched.
        let Some((adventure_id, acts, has_opening)) = self
            .extras
            .borrow()
            .as_ref()
            .map(|extras| (extras.adventure_id, extras.acts.clone(), extras.has_opening))
        else {
            log::error!("the builder view was requested before the page was opened");
            return;
        };

        let Some(adventure) = self
            .adventures
            .current_adventure()
            .filter(|adventure| adventure.id == adventure_id)
        else {
            log::error!(
                "the snapshot does not hold the adventure the builder shows ({adventure_id})"
            );
            return;
        };

        let builder = ui.global::<BuilderGlobals>();
        builder.set_builder(viewmodels::builder_view(
            Some(&adventure),
            &acts,
            has_opening,
        ));
        if let Some(section) = section {
            builder.set_section(section.into());
        }
    }

    /// The builder page became visible again: render it and switch to it. The data is
    /// already cached, so this does not touch the database.
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

    /// The adventure the builder shows, taken from the snapshot rather than the cache.
    pub fn adventure(&self) -> Option<Adventure> {
        let adventure_id = self.extras.borrow().as_ref()?.adventure_id;
        self.adventures
            .current_adventure()
            .filter(|adventure| adventure.id == adventure_id)
    }

    /// The acts the page already knows about, so a write does not have to read them.
    pub fn acts(&self) -> Vec<Act> {
        self.extras
            .borrow()
            .as_ref()
            .map(|extras| extras.acts.clone())
            .unwrap_or_default()
    }

    /// Re-reads the acts, the only extras a write can invalidate. One small query.
    pub fn reload_acts(&self) {
        let Some(adventure_id) = self
            .extras
            .borrow()
            .as_ref()
            .map(|extras| extras.adventure_id)
        else {
            return;
        };

        let result = tokio::task::block_in_place(|| {
            self.handle.block_on(self.adventures.get_acts(adventure_id))
        });

        match result {
            Ok(acts) => {
                if let Some(extras) = self.extras.borrow_mut().as_mut() {
                    extras.acts = acts;
                }
            }
            Err(err) => log::error!("could not reload the acts of {adventure_id}: {err}"),
        }
    }

    /// An opening now exists, which nothing else in the snapshot tracks.
    pub fn mark_opening_written(&self) {
        if let Some(extras) = self.extras.borrow_mut().as_mut() {
            extras.has_opening = true;
        }
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
