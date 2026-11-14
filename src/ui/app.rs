use std::sync::Arc;

use iced::{Element, Task};

use crate::app::AppContext;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::config::Config;
use crate::provision::{Gpu, RunningEngines};
use crate::ui::pages::{Event, Page, chat, main_hub, setup};

pub type Rebuild<C, N, M, A, P, I> =
    Arc<dyn Fn(&Config) -> Result<AppContext<C, N, M, A, P, I>, String> + Send + Sync>;

#[derive(Debug, Clone)]
pub enum Message {
    MainHub(main_hub::Message),
    Chat(chat::Message),
    Setup(setup::Message),
}

pub struct App<
    C: ChatPort + Clone + 'static,
    N: NarratorPort + Clone + 'static,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
    P: PromptPort + Clone + 'static,
    I: ImagePort + Clone + 'static,
> {
    page: Page,
    main_hub: main_hub::State,
    chat: chat::State,
    setup: setup::State,
    rebuild: Rebuild<C, N, M, A, P, I>,
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
> App<C, N, M, A, P, I>
{
    pub fn boot(
        ctx: AppContext<C, N, M, A, P, I>,
        rebuild: Rebuild<C, N, M, A, P, I>,
        config: Config,
        gpu: Gpu,
        open_setup: bool,
    ) -> (Self, Task<Message>) {
        let handle = tokio::runtime::Handle::current();

        let (main_hub, hub_task) = main_hub::State::boot(ctx.adventure_manager, handle.clone());
        let (chat, _chat_task) = chat::State::boot();
        let (setup, setup_task) = setup::State::boot(config, gpu, handle.clone());

        let page = if open_setup || setup.needs_answer() {
            Page::Setup
        } else {
            Page::MainHub
        };

        let app = Self {
            page,
            main_hub,
            chat,
            setup,
            rebuild,
            _engines: None,
        };

        (
            app,
            Task::batch([
                hub_task.map(Message::MainHub),
                setup_task.map(Message::Setup),
            ]),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainHub(message) => {
                let outcome = self.main_hub.update(message);
                self.route(outcome, Message::MainHub)
            }
            Message::Chat(message) => {
                let outcome = self.chat.update(message);
                self.route(outcome, Message::Chat)
            }
            Message::Setup(message) => {
                let outcome = self.setup.update(message);
                self.route(outcome, Message::Setup)
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.page {
            Page::MainHub => self.main_hub.view().map(Message::MainHub),
            Page::Chat => self.chat.view().map(Message::Chat),
            Page::Setup => self.setup.view().map(Message::Setup),
        }
    }

    fn route<T: Send + 'static>(
        &mut self,
        outcome: (Task<T>, Event),
        wrap: fn(T) -> Message,
    ) -> Task<Message> {
        let (task, event) = outcome;
        Task::batch([task.map(wrap), self.dispatch(event)])
    }

    fn dispatch(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::None => Task::none(),
            Event::Show(page) => self.show(page),
            Event::EnginesReady { engines, config } => self.engines_ready(engines, &config),
        }
    }

    fn show(&mut self, page: Page) -> Task<Message> {
        self.page = page;

        match page {
            Page::MainHub => {
                let outcome = self.main_hub.update(main_hub::Message::Refresh);
                self.route(outcome, Message::MainHub)
            }
            Page::Chat | Page::Setup => Task::none(),
        }
    }

    fn engines_ready(&mut self, engines: Arc<RunningEngines>, config: &Config) -> Task<Message> {
        log::info!("engines ready: {engines:?}");
        self._engines = Some(engines);

        match (self.rebuild)(config) {
            Ok(ctx) => self.main_hub.rebind(ctx.adventure_manager),
            Err(err) => log::error!("could not point the app at the new engines: {err}"),
        }

        if self.page == Page::Setup {
            return self.show(Page::MainHub);
        }
        Task::none()
    }
}
