use std::sync::Arc;

use iced::{Element, Event as InputEvent, Size, Task, keyboard, mouse, window};

use crate::app::AppContext;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::config::Config;
use crate::provision::{Gpu, RunningEngines};
use crate::ui::layout;
use crate::ui::pages::{Event, Page, builder, chat, main_hub, setup};

pub type Rebuild<C, N, M, A, P, I> =
    Arc<dyn Fn(&Config) -> Result<AppContext<C, N, M, A, P, I>, String> + Send + Sync>;

#[derive(Debug, Clone)]
pub enum Message {
    MainHub(main_hub::Message),
    Builder(builder::Message),
    Chat(chat::Message),
    Setup(setup::Message),
    WindowResized(Size),
    ModifiersChanged(keyboard::Modifiers),
    Wheel(mouse::ScrollDelta),
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
    builder: builder::State<P, I>,
    chat: chat::State<C, N, M, A>,
    setup: setup::State,
    rebuild: Rebuild<C, N, M, A, P, I>,
    auto_scale: f32,
    zoom: f32,
    window_width: f32,
    ctrl_held: bool,
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

        let (chat, _chat_task) = chat::State::boot(
            ctx.adventure_manager.clone(),
            ctx.message_manager.clone(),
            handle.clone(),
        );
        let (main_hub, hub_task) =
            main_hub::State::boot(ctx.adventure_manager.clone(), handle.clone());
        let (builder, _builder_task) = builder::State::boot(
            ctx.adventure_manager.clone(),
            ctx.character_manager.clone(),
            handle.clone(),
        );
        let (setup, setup_task) = setup::State::boot(config, gpu, handle.clone());

        let page = if open_setup || setup.needs_answer() {
            Page::Setup
        } else {
            Page::MainHub
        };

        let app = Self {
            page,
            main_hub,
            builder,
            chat,
            setup,
            rebuild,
            auto_scale: 1.0,
            zoom: 1.0,
            window_width: layout::DEFAULT_WINDOW.width,
            ctrl_held: false,
            _engines: None,
        };

        (
            app,
            Task::batch([
                hub_task.map(Message::MainHub),
                setup_task.map(Message::Setup),
                window::latest()
                    .and_then(window::size)
                    .map(Message::WindowResized),
            ]),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainHub(message) => {
                let outcome = self.main_hub.update(message);
                self.route(outcome, Message::MainHub)
            }
            Message::Builder(message) => {
                let outcome = self.builder.update(message);
                self.route(outcome, Message::Builder)
            }
            Message::Chat(message) => {
                let outcome = self.chat.update(message);
                self.route(outcome, Message::Chat)
            }
            Message::Setup(message) => {
                let outcome = self.setup.update(message);
                self.route(outcome, Message::Setup)
            }
            Message::WindowResized(size) => {
                self.resized(size);
                Task::none()
            }
            Message::ModifiersChanged(modifiers) => {
                self.ctrl_held = modifiers.control();
                Task::none()
            }
            Message::Wheel(delta) => {
                if self.ctrl_held {
                    self.zoom = layout::zoomed(self.zoom, delta);
                    self.apply_layout();
                }
                Task::none()
            }
        }
    }

    pub fn scale_factor(&self) -> f32 {
        self.auto_scale * self.zoom
    }

    fn resized(&mut self, reported: Size) {
        self.window_width = reported.width * self.scale_factor();
        self.auto_scale = layout::scale_for(self.window_width);
        self.apply_layout();
    }

    fn apply_layout(&mut self) {
        let width = self.window_width / self.scale_factor();
        self.main_hub.set_window_width(width);
        self.chat.set_window_width(width);
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.page {
            Page::MainHub => self.main_hub.view().map(Message::MainHub),
            Page::Builder => self.builder.view().map(Message::Builder),
            Page::Chat => self.chat.view().map(Message::Chat),
            Page::Setup => self.setup.view().map(Message::Setup),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let resizes = iced::event::listen_with(input);

        match self.page {
            Page::Setup => {
                iced::Subscription::batch([self.setup.subscription().map(Message::Setup), resizes])
            }
            Page::MainHub | Page::Builder | Page::Chat => resizes,
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
            Event::OpenBuilder(adventure_id) => {
                self.page = Page::Builder;
                self.builder.open(adventure_id).map(Message::Builder)
            }
            Event::OpenChat(adventure_id) => {
                self.page = Page::Chat;
                self.chat.open(adventure_id).map(Message::Chat)
            }
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
            Page::Builder | Page::Chat | Page::Setup => Task::none(),
        }
    }

    fn engines_ready(&mut self, engines: Arc<RunningEngines>, config: &Config) -> Task<Message> {
        log::info!("engines ready: {engines:?}");
        self._engines = Some(engines);

        match (self.rebuild)(config) {
            Ok(ctx) => {
                self.chat
                    .rebind(ctx.adventure_manager.clone(), ctx.message_manager.clone());
                self.builder
                    .rebind(ctx.adventure_manager.clone(), ctx.character_manager.clone());
                self.main_hub.rebind(ctx.adventure_manager);
            }
            Err(err) => log::error!("could not point the app at the new engines: {err}"),
        }

        if self.page == Page::Setup {
            return self.show(Page::MainHub);
        }
        Task::none()
    }
}

fn input(event: InputEvent, _status: iced::event::Status, _window: window::Id) -> Option<Message> {
    match event {
        InputEvent::Window(window::Event::Resized(size)) => Some(Message::WindowResized(size)),
        InputEvent::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
            Some(Message::ModifiersChanged(modifiers))
        }
        InputEvent::Mouse(mouse::Event::WheelScrolled { delta }) => Some(Message::Wheel(delta)),
        _ => None,
    }
}
