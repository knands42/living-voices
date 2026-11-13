use std::sync::Arc;

use iced::{Element, Size, Task, Theme};

use crate::app::AppContext;
use crate::backend::application::adventure_manager::AdventureManager;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::backend::domain::adventure::AdventureSummary;
use crate::config::Config;
use crate::provision::{Gpu, RunningEngines};
use crate::ui::pages::setup::{self, SetupState};
use crate::ui::pages::{chat, main_hub};
use crate::ui::task;

type Rebuild<C, N, M, A, P, I> =
    Arc<dyn Fn(&Config) -> Result<AppContext<C, N, M, A, P, I>, String> + Send + Sync>;

pub fn run<
    C: ChatPort + Clone + 'static,
    N: NarratorPort + Clone + 'static,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
    P: PromptPort + Clone + 'static,
    I: ImagePort + Clone + 'static,
>(
    ctx: AppContext<C, N, M, A, P, I>,
    rebuild: impl Fn(&Config) -> Result<AppContext<C, N, M, A, P, I>, String> + Send + Sync + 'static,
    config: Config,
    gpu: Gpu,
    open_setup: bool,
) -> iced::Result {
    let rebuild: Rebuild<C, N, M, A, P, I> = Arc::new(rebuild);

    iced::application(
        move || {
            State::boot(
                ctx.clone(),
                rebuild.clone(),
                config.clone(),
                gpu,
                open_setup,
            )
        },
        State::update,
        State::view,
    )
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
    Setup,
}

#[derive(Debug, Clone)]
pub enum Message {
    PageRequested(Page),
    AdventureSummaryLoaded(Vec<AdventureSummary>),
    Setup(setup::Message),
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
    rebuild: Rebuild<C, N, M, A, P, I>,
    handle: tokio::runtime::Handle,
    adventure_summaries: Vec<AdventureSummary>,
    setup: SetupState,
    // Dropping this kills the engine processes, so it lives exactly as long as the window.
    _engines: Option<Arc<RunningEngines>>,
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
    fn boot(
        ctx: AppContext<C, N, M, A, P, I>,
        rebuild: Rebuild<C, N, M, A, P, I>,
        config: Config,
        gpu: Gpu,
        open_setup: bool,
    ) -> (Self, Task<Message>) {
        let handle = tokio::runtime::Handle::current();
        let mut setup = SetupState::new(config, gpu, handle.clone());

        // A first run waits for the player's answer; any other run starts the engines now.
        let engines = if setup.needs_answer() {
            Task::none()
        } else {
            setup.start().map(Message::Setup)
        };
        let summaries = load_summaries(handle.clone(), ctx.adventures.clone());

        let state = Self {
            page: if open_setup || setup.needs_answer() {
                Page::Setup
            } else {
                Page::MainHub
            },
            ctx,
            rebuild,
            handle,
            adventure_summaries: Vec::new(),
            setup,
            _engines: None,
        };

        (state, Task::batch([summaries, engines]))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AdventureSummaryLoaded(list) => {
                self.adventure_summaries = list;
                Task::none()
            }
            Message::PageRequested(page) => self.show(page),
            Message::Setup(message) => {
                let (task, event) = self.setup.update(message);
                let follow_up = match event {
                    setup::Event::None => Task::none(),
                    setup::Event::BackRequested => self.show(Page::MainHub),
                    setup::Event::EnginesReady { engines, config } => {
                        self.engines_ready(engines, &config)
                    }
                };
                Task::batch([task.map(Message::Setup), follow_up])
            }
        }
    }

    fn engines_ready(&mut self, engines: Arc<RunningEngines>, config: &Config) -> Task<Message> {
        log::info!("engines ready: {engines:?}");
        self._engines = Some(engines);

        match (self.rebuild)(config) {
            Ok(ctx) => self.ctx = ctx,
            Err(err) => log::error!("could not point the app at the new engines: {err}"),
        }

        if self.page == Page::Setup {
            return self.show(Page::MainHub);
        }
        Task::none()
    }

    fn show(&mut self, page: Page) -> Task<Message> {
        self.page = page;

        // Coming back to the hub re-reads the list, so a new adventure shows up without a reload.
        if page == Page::MainHub {
            return load_summaries(self.handle.clone(), self.ctx.adventures.clone());
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        match self.page {
            Page::MainHub => main_hub::view(&self.adventure_summaries),
            Page::Chat => chat::view(),
            Page::Setup => setup::view(&self.setup).map(Message::Setup),
        }
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
