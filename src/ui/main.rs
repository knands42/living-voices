use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length, Size, Task, Theme};

use crate::app::AppContext;
use crate::backend::application::adventure_manager::AdventureManager;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::backend::domain::adventure::AdventureSummary;
use crate::ui::pages::{chat, main_hub};
use crate::ui::task;

pub fn run<
    C: ChatPort + Clone + 'static,
    N: NarratorPort + Clone + 'static,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
    P: PromptPort + Clone + 'static,
    I: ImagePort + Clone + 'static,
>(
    ctx: AppContext<C, N, M, A, P, I>,
) -> iced::Result {
    iced::application(move || State::boot(ctx.clone()), State::update, State::view)
        .title("Living Voices")
        .theme(|_: &State<C, N, M, A, P, I>| Theme::CatppuccinMacchiato)
        .window_size(Size::new(1360.0, 900.0))
        .antialiasing(true)
        .run()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    MainHub,
    Chat,
}

#[derive(Debug, Clone)]
pub enum Message {
    PageRequested(Page),
    AdventureSummaryLoaded(Vec<AdventureSummary>),
}

pub struct State<
    C: ChatPort + Clone + 'static,
    N: NarratorPort + Clone + 'static,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
    P: PromptPort + Clone + 'static,
    I: ImagePort + Clone + 'static,
> {
    page: Page,
    ctx: AppContext<C, N, M, A, P, I>,
    handle: tokio::runtime::Handle,
    adventure_summaries: Vec<AdventureSummary>,
}

impl<
    C: ChatPort + Clone + 'static,
    N: NarratorPort + Clone + 'static,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
    P: PromptPort + Clone + 'static,
    I: ImagePort + Clone + 'static,
> State<C, N, M, A, P, I>
{
    /// The one place the window starts from, so `main` never builds UI state.
    fn boot(ctx: AppContext<C, N, M, A, P, I>) -> (Self, Task<Message>) {
        let adventures = ctx.adventures.clone();
        let handle = tokio::runtime::Handle::current();

        let state = Self {
            page: Page::MainHub,
            ctx,
            handle: handle.clone(),
            adventure_summaries: Vec::new(),
        };

        (state, load_summaries(handle, adventures))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AdventureSummaryLoaded(list) => {
                self.adventure_summaries = list;
                Task::none()
            }
            Message::PageRequested(page) => {
                self.page = page;

                if page == Page::MainHub {
                    return load_summaries(self.handle.clone(), self.ctx.adventures.clone());
                }

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match self.page {
            Page::MainHub => main_hub::view(&self.adventure_summaries),
            Page::Chat => chat::view(),
        };

        container(
            column![
                row![
                    text("Living Voices").size(28),
                    Space::new().width(Length::Fill),
                    self.tabs(),
                ]
                .align_y(Alignment::Center)
                .spacing(16),
                content,
            ]
            .spacing(24),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// The switch every page shares: a new page needs a `Page` variant and a row
    /// here, nothing else.
    fn tabs(&self) -> Element<'_, Message> {
        let pages = [("Adventures", Page::MainHub), ("Chat", Page::Chat)];

        pages
            .into_iter()
            .fold(row![].spacing(8), |row, (label, page)| {
                let tab = button(text(label)).on_press(Message::PageRequested(page));
                row.push(if self.page == page {
                    tab.style(button::primary)
                } else {
                    tab.style(button::secondary)
                })
            })
            .into()
    }
}

fn load_summaries(handle: tokio::runtime::Handle, adventures: AdventureManager) -> Task<Message> {
    Task::perform(
        task::on_tokio(handle, async move {
            adventures.get_adventures_summaries().await
        }),
        |result| match result {
            Some(Ok(summaries)) => Message::AdventureSummaryLoaded(summaries),
            Some(Err(err)) => {
                log::error!("could not load adventure summaries: {err}");
                Message::AdventureSummaryLoaded(Vec::new())
            }
            None => Message::AdventureSummaryLoaded(Vec::new()),
        },
    )
}
