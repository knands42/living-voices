use std::future::Future;

use crate::app::DefaultContext;
use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::application::character_gallery::CharacterGallery;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::Act;
use crate::backend::domain::message::Message;
use crate::ui::callbacks::parse_id;
use crate::ui::{
    ActInputView, BuilderGlobals, MainGlobals, RootWindow, StoryInputView, viewmodels,
};
use slint::ComponentHandle;
use thiserror::Error;
use tokio::runtime::Handle;
use uuid::Uuid;

#[derive(Error, Debug)]
enum BuilderPageError {
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Adventure(#[from] AdventureManagerError),
}

/// Everything a builder-page callback needs, cheap to clone into each closure.
///
/// There is no cache here: the backend snapshot (`AppState`) is the only copy, and every
/// write keeps it current (`load_adventure`, `update_adventure`, `upsert_act`,
/// `create_initial_messages`). Rendering therefore costs no I/O and there is never a
/// second copy to reconcile.
///
/// The rule: fetch when the page is entered, render from the snapshot everywhere else.
/// Saves write and then re-render; they never re-read.
#[derive(Clone)]
struct BuilderPageContext {
    handle: Handle,
    adventures: AdventureManager,
    gallery: CharacterGallery,
    ui_weak: slint::Weak<RootWindow>,
}

impl BuilderPageContext {
    /// The one place that reads the database for the builder: it warms the snapshot
    /// with the adventure, its acts and whether the opening exists.
    fn open(&self, adventure_id: Uuid) -> Result<(), BuilderPageError> {
        self.block_on(self.adventures.load_adventure(adventure_id))?;

        self.show(Some("story"));
        self.reveal();

        Ok(())
    }

    /// Pushes the builder view into Slint: the adventure comes from the snapshot, and the
    /// two lists the view shows are read here, as small indexed queries.
    fn show(&self, section: Option<&str>) {
        let Some(ui) = self.ui_weak.upgrade() else {
            return;
        };
        let Some(adventure) = self.adventures.current_adventure() else {
            log::error!("the builder view was requested before an adventure was loaded");
            return;
        };

        let lists = self.block_on(async {
            let acts = self.adventures.get_acts(adventure.id).await?;
            let opening = self.adventures.get_messages(adventure.id, 10).await?;
            Ok::<_, AdventureManagerError>((acts, !opening.is_empty()))
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
    fn enter(&self) {
        let section = self
            .ui_weak
            .upgrade()
            .map(|ui| ui.global::<BuilderGlobals>().get_section().to_string())
            .filter(|section| !section.is_empty())
            .unwrap_or_else(|| "story".to_string());

        self.show(Some(section.as_str()));
        self.reveal();
    }

    /// The story fields live on the adventure, which the snapshot already holds, so this
    /// reads memory and writes once. The write refreshes the snapshot in turn.
    fn save_story(
        &self,
        adventure_id: Uuid,
        input: StoryInputView,
    ) -> Result<(), BuilderPageError> {
        let title = input.title.trim();
        if title.is_empty() {
            return Err(BuilderPageError::Invalid("title"));
        }

        let mut adventure = self.adventure(adventure_id)?;
        adventure.title = title.to_string();
        adventure.headline = input.headline.trim().to_string();
        adventure.tone = input.tone.trim().to_string();
        adventure.lore.text = input.lore.trim().to_string();
        if !input.narrator_name.trim().is_empty() {
            adventure.narrator.name = input.narrator_name.trim().to_string();
        }

        self.block_on(self.adventures.update_adventure(&adventure))
            .map(|_| ())
    }

    fn save_act(&self, adventure_id: Uuid, input: ActInputView) -> Result<(), BuilderPageError> {
        let goal = input.goal.trim();
        if goal.is_empty() {
            return Err(BuilderPageError::Invalid("act goal"));
        }
        let trigger = Some(input.trigger.trim())
            .filter(|trigger| !trigger.is_empty())
            .map(str::to_string);

        let acts = self.block_on(self.adventures.get_acts(adventure_id))?;
        let mut act = if input.id.is_empty() {
            let ordinal = acts.iter().map(|act| act.ordinal + 1).max().unwrap_or(0);
            Act::new(adventure_id, ordinal, goal)
        } else {
            let act_id = parse_id(input.id.as_str()).ok_or(BuilderPageError::Invalid("act id"))?;
            acts.into_iter()
                .find(|act| act.id == act_id)
                .ok_or(BuilderPageError::Invalid("act id"))?
        };
        act.goal = goal.to_string();
        act.trigger = trigger;

        self.block_on(self.adventures.upsert_act(&act))
    }

    fn save_opening(&self, adventure_id: Uuid, text: &str) -> Result<(), BuilderPageError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(BuilderPageError::Invalid("opening"));
        }

        let adventure = self.adventure(adventure_id)?;
        let message = adventure.message_from_reply(Message::NARRATOR_SPEAKER_NAME, text);

        self.block_on(self.adventures.create_initial_messages(&[message]))
    }

    /// The adventure the page edits, read from the snapshot rather than the database.
    fn adventure(&self, adventure_id: Uuid) -> Result<Adventure, BuilderPageError> {
        self.adventures
            .current_adventure()
            .filter(|adventure| adventure.id == adventure_id)
            .ok_or(BuilderPageError::Invalid("adventure"))
    }

    /// Callbacks run on the UI thread, so blocking on the runtime keeps the ordering of
    /// a save and the render that follows it.
    fn block_on<T>(
        &self,
        work: impl Future<Output = Result<T, AdventureManagerError>>,
    ) -> Result<T, BuilderPageError> {
        tokio::task::block_in_place(|| self.handle.block_on(work)).map_err(BuilderPageError::from)
    }

    fn reveal(&self) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_current_page("builder".into());
        }
    }
}

pub fn register_builder_page(ui: &RootWindow, ctx: &DefaultContext) {
    let env = BuilderPageContext {
        handle: Handle::current(),
        adventures: ctx.adventures.clone(),
        gallery: ctx.characters.gallery().clone(),
        ui_weak: ui.as_weak(),
    };

    register_open(ui, &env);
    register_story(ui, &env);
    register_act(ui, &env);
    register_opening(ui, &env);
    register_section(ui, &env);
}

/// Main page -> builder: a draft opens the builder on its story section.
fn register_open(ui: &RootWindow, env: &BuilderPageContext) {
    let env = env.clone();
    ui.global::<MainGlobals>()
        .on_draft_adventure_selected(move |id| {
            let Some(adventure_id) = parse_id(id.as_str()) else {
                return;
            };

            if let Err(err) = env.open(adventure_id) {
                log::error!("could not open the draft {adventure_id}: {err}");
            }
        });
}

/// Builder page, story section: every save writes once, then re-renders.
fn register_story(ui: &RootWindow, env: &BuilderPageContext) {
    let env = env.clone();
    ui.global::<BuilderGlobals>()
        .on_save_story(move |adventure_id, input| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            match env.save_story(adventure_id, input) {
                Ok(()) => env.show(None),
                Err(err) => log::error!("could not save the story of {adventure_id}: {err}"),
            }
        });
}

/// Builder page, acts section: an act with an empty id is a new one, otherwise it is the
/// act being edited.
fn register_act(ui: &RootWindow, env: &BuilderPageContext) {
    let env = env.clone();
    ui.global::<BuilderGlobals>()
        .on_save_act(move |adventure_id, input| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            match env.save_act(adventure_id, input) {
                Ok(()) => env.show(None),
                Err(err) => log::error!("could not save an act of {adventure_id}: {err}"),
            }
        });
}

/// Builder page, opening section: the opening is the adventure's first narrator message.
fn register_opening(ui: &RootWindow, env: &BuilderPageContext) {
    let env = env.clone();
    ui.global::<BuilderGlobals>()
        .on_save_opening(move |adventure_id, text| {
            let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
                return;
            };

            match env.save_opening(adventure_id, text.as_str()) {
                Ok(()) => env.show(None),
                Err(err) => log::error!("could not save the opening of {adventure_id}: {err}"),
            }
        });
}

/// Builder page: the sidebar only requests a section, it never mutates navigation.
fn register_section(ui: &RootWindow, env: &BuilderPageContext) {
    let section_env = env.clone();
    ui.global::<BuilderGlobals>()
        .on_section_requested(move |section| section_env.show(Some(section.as_str())));

    let entered_env = env.clone();
    ui.global::<BuilderGlobals>()
        .on_builder_entered(move || entered_env.enter());
}
