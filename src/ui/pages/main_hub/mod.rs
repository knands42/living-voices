mod body;
mod header;

use chrono::{DateTime, Utc};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length, Task};

use crate::backend::application::adventure_manager::AdventureManager;
use crate::backend::domain::adventure::{Adventure, AdventureSummary, OngoingAdventure};
use crate::ui::pages::main_hub::Message::CatalogLoaded;
use crate::ui::pages::main_hub::body::{body, new_adventure_popup};
use crate::ui::pages::main_hub::header::header;
use crate::ui::pages::{Event, Page};
use crate::ui::task::on_tokio;
use crate::ui::theme;

const SAMPLE: [(&str, &str, &str); 3] = [
    (
        "The Gilded Cage",
        "A hostage-wrapped court intrigue in a city that sells its own heirs.",
        "in progress",
    ),
    (
        "Salt and Ash",
        "A fishing town stopped answering the mainland three winters ago.",
        "draft",
    ),
    (
        "Where the Lanterns Go",
        "Every road out of the valley leads back to the same inn.",
        "draft",
    ),
];

#[derive(Debug, Clone, Default)]
pub struct AdventureCreationDraft {
    title: String,
    headline: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    CatalogLoaded(Catalogue),
    SetupRequested,
    ChatRequested,
    SearchChanged(String),

    AdventureDraftClosedNoop,
    AdventureDraftChanged(AdventureCreationDraft),
    AdventureDraftRequested,
    AdventureDraftCancelled,
    AdventureDraftSubmitted,
}

#[derive(Debug, Clone, Default)]
pub struct Catalogue {
    adventures: Vec<AdventureSummary>,
    shelf: Vec<OngoingAdventure>,
}

pub struct State {
    catalogue: Catalogue,
    handle: tokio::runtime::Handle,
    query: String,

    adventure_manager: AdventureManager,

    adventure_draft_open: bool,
    new_adventure_draft: AdventureCreationDraft,
}

impl State {
    pub fn boot(
        adventure_manager: AdventureManager,
        handle: tokio::runtime::Handle,
    ) -> (Self, Task<Message>) {
        let adventure_summaries = adventure_manager.get_adventures_summaries();
        let shelf = adventure_manager.get_ongoing_adventures();
        let catalog = Catalogue {
            adventures: adventure_summaries,
            shelf
        };

        let state = Self {
            catalogue: catalog,
            handle,
            adventure_manager,
            query: String::new(),
            adventure_draft_open: false,
            new_adventure_draft: Default::default(),
        };
        let task = state.load();
        (state, task)
    }

    pub fn rebind(&mut self, adventure_manager: AdventureManager) {
        self.adventure_manager = adventure_manager;
        let _ = self.load();
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::Refresh => return (self.load(), Event::None),
            Message::CatalogLoaded(catalog) => self.catalogue = catalog,
            Message::SetupRequested => return (Task::none(), Event::Show(Page::Setup)),
            Message::ChatRequested => return (Task::none(), Event::Show(Page::Chat)),
            Message::SearchChanged(query) => self.query = query,

            Message::AdventureDraftClosedNoop => self.adventure_draft_open = false,
            Message::AdventureDraftRequested => {
                self.adventure_draft_open = true;
                self.new_adventure_draft = AdventureCreationDraft::default()
            }
            Message::AdventureDraftChanged(draft) => self.new_adventure_draft = draft,
            Message::AdventureDraftCancelled => {
                self.new_adventure_draft = AdventureCreationDraft::default()
            }
            Message::AdventureDraftSubmitted => {
                // TODO: create adventure
            }
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
            iced::widget::Stack::with_children([page, new_adventure_popup(self)]).into()
        } else {
            page
        }
    }

    fn load(&self) -> Task<Message> {
        let handle = self.handle.clone();
        let adventure_manager = self.adventure_manager.clone();
        
        let mut catalogue = Catalogue::default();

        Task::perform(
            on_tokio(handle.clone(), async move { 
                adventure_manager.get_adventures_summaries().await 
            }),
            |result| match result {
                Some(Ok(summaries)) => catalogue.adventures = summaries,
                Some(Err(err)) => {
                    log::error!("could not load adventure summaries: {err}");
                }
                None => {
                    log::info!("no adventures available at the moment")
                },
            },
        );
        
        Task::perform(
            on_tokio(handle, async move { 
                adventure_manager.get_ongoing_adventures().await 
            }),
            |result| match result {
                Some(Ok(shelf)) => catalogue.shelf = shelf,
                Some(Err(err)) => {
                    log::error!("could not load ongoing adventures: {err}");
                }
                None => {
                    log::info!("no ongoing adventures available at the moment")
                },
            },
        )
    }
}
