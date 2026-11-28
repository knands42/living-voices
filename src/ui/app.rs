use std::sync::Arc;

use iced::{Element, Event as InputEvent, Size, Task, keyboard, mouse, window};

use crate::app::AppContext;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::config::ConfigHandle;
use crate::provision::llama::llama_server::LlamaServer;
use crate::provision::{Gpu, configure_memory_engine, start_memory_engine};
use crate::ui::layout;
use crate::ui::pages::{Event, Page, builder, chat, main_hub, setup};
use crate::ui::task::on_tokio;

#[derive(Debug, Clone)]
pub enum Message {
    MainHub(main_hub::Message),
    Builder(builder::Message),
    Chat(chat::Message),
    Setup(setup::Message),
    MemoryEngine(Option<Result<Arc<LlamaServer>, String>>),
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
    config: ConfigHandle,
    handle: tokio::runtime::Handle,
    memory_engine: Option<Arc<LlamaServer>>,
    memory_starting: bool,
    auto_scale: f32,
    zoom: f32,
    window_width: f32,
    ctrl_held: bool,
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
        config: ConfigHandle,
        gpu: Gpu,
        open_setup: bool,
    ) -> (Self, Task<Message>) {
        let handle = tokio::runtime::Handle::current();

        let (chat, _chat_task) = chat::State::boot(
            ctx.adventure_manager.clone(),
            ctx.message_manager.clone(),
            config.clone(),
            handle.clone(),
        );
        let (main_hub, hub_task) =
            main_hub::State::boot(ctx.adventure_manager.clone(), handle.clone());
        let (builder, _builder_task) = builder::State::boot(
            ctx.adventure_manager.clone(),
            ctx.character_manager.clone(),
            handle.clone(),
        );
        let (setup, setup_task) = setup::State::boot(config.clone(), gpu, handle.clone());

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
            config,
            handle,
            memory_engine: None,
            memory_starting: false,
            auto_scale: 1.0,
            zoom: 1.0,
            window_width: layout::DEFAULT_WINDOW.width,
            ctrl_held: false,
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
            Message::MemoryEngine(outcome) => {
                self.memory_starting = false;
                match outcome {
                    Some(Ok(engine)) => {
                        configure_memory_engine(&self.config, &engine);
                        self.memory_engine = Some(engine);
                    }
                    Some(Err(err)) => log::warn!("memory engine unavailable: {err}"),
                    None => log::warn!("the memory engine task was cancelled"),
                }
                Task::none()
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
                Task::batch([
                    self.chat.open(adventure_id).map(Message::Chat),
                    self.ensure_memory_engine(),
                ])
            }
        }
    }

    fn ensure_memory_engine(&mut self) -> Task<Message> {
        if self.memory_engine.is_some() || self.memory_starting {
            return Task::none();
        }

        self.memory_starting = true;
        Task::perform(
            on_tokio(self.handle.clone(), async move {
                start_memory_engine().await.map(Arc::new)
            }),
            Message::MemoryEngine,
        )
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
