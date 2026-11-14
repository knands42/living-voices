mod body;
mod header;

use iced::widget::{Stack, column, container};
use iced::{Element, Length, Task};
use uuid::Uuid;

use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::domain::adventure::{Adventure, AdventureSummary, OngoingAdventure};
use crate::ui::pages::main_hub::body::{body, new_adventure_popup};
use crate::ui::pages::main_hub::header::header;
use crate::ui::pages::{Event, Page};
use crate::ui::task::on_tokio;
use crate::ui::theme;

#[derive(Debug, Clone, Default)]
pub struct AdventureCreationDraft {
    title: String,
    headline: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    AdventuresLoaded(Vec<AdventureSummary>),
    ShelfLoaded(Vec<OngoingAdventure>),
    SetupRequested,
    ChatRequested,
    OpenAdventure(Uuid),
    SearchChanged(String),
    TagSelected(Option<String>),
    AdventureDraftRequested,
    AdventureDraftTitleChanged(String),
    AdventureDraftHeadlineChanged(String),
    AdventureDraftCancelled,
    AdventureDraftSubmitted,
    AdventureDraftCreated(Result<Adventure, String>),
    Noop,
}

#[derive(Debug, Clone, Default)]
pub struct Catalogue {
    adventures: Vec<AdventureSummary>,
    shelf: Vec<OngoingAdventure>,
}

pub struct State {
    catalogue: Catalogue,
    handle: tokio::runtime::Handle,
    adventure_manager: AdventureManager,
    query: String,
    active_tag: Option<String>,
    adventure_draft_open: bool,
    new_adventure_draft: AdventureCreationDraft,
}

impl State {
    pub fn boot(
        adventure_manager: AdventureManager,
        handle: tokio::runtime::Handle,
    ) -> (Self, Task<Message>) {
        let state = Self {
            catalogue: Catalogue::default(),
            handle,
            adventure_manager,
            query: String::new(),
            active_tag: None,
            adventure_draft_open: false,
            new_adventure_draft: AdventureCreationDraft::default(),
        };
        let task = state.load();
        (state, task)
    }

    pub fn rebind(&mut self, adventure_manager: AdventureManager) {
        self.adventure_manager = adventure_manager;
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::Refresh => return (self.load(), Event::None),
            Message::AdventuresLoaded(adventures) => self.catalogue.adventures = adventures,
            Message::ShelfLoaded(shelf) => self.catalogue.shelf = shelf,
            Message::SetupRequested => return (Task::none(), Event::Show(Page::Setup)),
            Message::ChatRequested => return (Task::none(), Event::Show(Page::Chat)),
            Message::OpenAdventure(adventure_id) => {
                log::info!("opening adventure {adventure_id}");
                return (Task::none(), Event::Show(Page::Chat));
            }
            Message::SearchChanged(query) => self.query = query,
            Message::TagSelected(tag) => self.active_tag = tag,
            Message::AdventureDraftRequested => {
                self.adventure_draft_open = true;
                self.new_adventure_draft = AdventureCreationDraft::default();
            }
            Message::AdventureDraftTitleChanged(title) => self.new_adventure_draft.title = title,
            Message::AdventureDraftHeadlineChanged(headline) => {
                self.new_adventure_draft.headline = headline;
            }
            Message::AdventureDraftCancelled => {
                self.adventure_draft_open = false;
                self.new_adventure_draft = AdventureCreationDraft::default();
            }
            Message::AdventureDraftSubmitted => {
                let draft = std::mem::take(&mut self.new_adventure_draft);
                self.adventure_draft_open = false;
                return (self.create_adventure(draft), Event::None);
            }
            Message::AdventureDraftCreated(result) => match result {
                Ok(_) => return (self.load(), Event::None),
                Err(err) => log::error!("could not create adventure: {err}"),
            },
            Message::Noop => {}
        }

        (Task::none(), Event::None)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let page = container(
            column![header(self), body(self)]
                .spacing(14.0)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding([18.0, 22.0])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::page)
        .into();

        if self.adventure_draft_open {
            Stack::with_children([page, new_adventure_popup(self)]).into()
        } else {
            page
        }
    }

    fn load(&self) -> Task<Message> {
        let handle = self.handle.clone();
        let adventure_manager = self.adventure_manager.clone();

        Task::batch([
            Task::perform(
                on_tokio(handle.clone(), {
                    let adventure_manager = adventure_manager.clone();
                    async move { adventure_manager.get_adventures_summaries().await }
                }),
                |result| Message::AdventuresLoaded(loaded(result, "adventure summaries")),
            ),
            Task::perform(
                on_tokio(handle, async move {
                    adventure_manager.get_ongoing_adventures().await
                }),
                |result| Message::ShelfLoaded(loaded(result, "ongoing adventures")),
            ),
        ])
    }

    fn create_adventure(&self, draft: AdventureCreationDraft) -> Task<Message> {
        let handle = self.handle.clone();
        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(handle, async move {
                adventure_manager
                    .create_adventure(draft.title, draft.headline, String::new())
                    .await
            }),
            |result| {
                let created = match result {
                    Some(Ok(adventure)) => Ok(adventure),
                    Some(Err(err)) => Err(err.to_string()),
                    None => Err("adventure creation task failed".to_string()),
                };
                Message::AdventureDraftCreated(created)
            },
        )
    }
}

fn loaded<T: Default>(result: Option<Result<T, AdventureManagerError>>, what: &'static str) -> T {
    match result {
        Some(Ok(items)) => items,
        Some(Err(err)) => {
            log::error!("could not load {what}: {err}");
            T::default()
        }
        None => T::default(),
    }
}
