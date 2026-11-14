use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length, Task};

use crate::backend::application::adventure_manager::AdventureManager;
use crate::backend::domain::adventure::AdventureSummary;
use crate::ui::pages::{Event, Page};
use crate::ui::task::on_tokio;

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

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    SummariesLoaded(Vec<AdventureSummary>),
    SetupRequested,
    ChatRequested,
}

pub struct State {
    adventures: AdventureManager,
    handle: tokio::runtime::Handle,
    summaries: Vec<AdventureSummary>,
}

impl State {
    pub fn boot(
        adventures: AdventureManager,
        handle: tokio::runtime::Handle,
    ) -> (Self, Task<Message>) {
        let state = Self {
            adventures,
            handle,
            summaries: Vec::new(),
        };
        let task = state.load();
        (state, task)
    }

    pub fn rebind(&mut self, adventures: AdventureManager) {
        self.adventures = adventures;
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::Refresh => return (self.load(), Event::None),
            Message::SummariesLoaded(summaries) => self.summaries = summaries,
            Message::SetupRequested => return (Task::none(), Event::Show(Page::Setup)),
            Message::ChatRequested => return (Task::none(), Event::Show(Page::Chat)),
        }

        (Task::none(), Event::None)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let entries = if self.summaries.is_empty() {
            entry_list(SAMPLE.into_iter())
        } else {
            entry_list(self.summaries.iter().map(|summary| {
                (
                    summary.title.as_str(),
                    summary.headline.as_str(),
                    summary.status.as_str(),
                )
            }))
        };

        column![
            row![
                text("Choose your adventure").size(24),
                Space::new().width(Length::Fill),
                button("New adventure"),
                button("Setup").on_press(Message::SetupRequested),
                button("Open chat").on_press(Message::ChatRequested),
            ]
            .align_y(Alignment::Center)
            .spacing(12),
            entries,
        ]
        .spacing(20)
        .into()
    }

    fn load(&self) -> Task<Message> {
        let handle = self.handle.clone();
        let adventures = self.adventures.clone();

        Task::perform(
            on_tokio(handle, async move {
                adventures.get_adventures_summaries().await
            }),
            |result| match result {
                Some(Ok(summaries)) => Message::SummariesLoaded(summaries),
                Some(Err(err)) => {
                    log::error!("could not load adventure summaries: {err}");
                    Message::SummariesLoaded(Vec::new())
                }
                None => Message::SummariesLoaded(Vec::new()),
            },
        )
    }
}

fn entry_list<'a>(
    entries: impl Iterator<Item = (&'a str, &'a str, &'a str)>,
) -> Element<'a, Message> {
    entries
        .fold(
            column![].spacing(12),
            |list, (title, description, status)| list.push(entry(title, description, status)),
        )
        .into()
}

fn entry<'a>(title: &'a str, description: &'a str, status: &'a str) -> Element<'a, Message> {
    container(
        row![
            column![text(title).size(20), text(description).size(14)].spacing(4),
            Space::new().width(Length::Fill),
            text(status).size(14),
        ]
        .align_y(Alignment::Center)
        .spacing(16),
    )
    .style(container::rounded_box)
    .padding(16)
    .width(Length::Fill)
    .into()
}
