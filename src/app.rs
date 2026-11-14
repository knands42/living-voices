use sqlx::SqlitePool;

use crate::ai::integrations::image::comfy_image::ComfyImage;
use crate::ai::integrations::text::chat_handler::ChatHandler;
use crate::ai::integrations::text::narrator_handler::NarratorHandler;
use crate::ai::{ActClose, MemoryHandler};
use crate::backend::application::character_gallery::CharacterGallery;
use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::image_port::{ImageError, ImagePort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::{
    backend::application::{
        adventure_manager::AdventureManager, character_manager::CharacterManager,
        memory_manager::MemoryManager, message_manager::MessageManager, state::SharedState,
    },
    config::{Config, Settings},
    provision::Gpu,
    ui,
};

#[derive(Clone)]
pub struct AppContext<
    C: ChatPort + Clone,
    N: NarratorPort + Clone,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
    P: PromptPort + Clone,
    I: ImagePort + Clone + 'static,
> {
    pub adventure_manager: AdventureManager,
    pub character_manager: CharacterManager<P, I>,
    pub message_manager: MessageManager<C, N, M, A>,
}

pub type DefaultContext =
    AppContext<ChatHandler, NarratorHandler, MemoryHandler, ActClose, ChatHandler, ComfyImage>;

pub struct App {
    pool: SqlitePool,
    state: SharedState,
    ctx: DefaultContext,
}

impl App {
    pub fn new(pool: SqlitePool, config: &Config) -> Result<Self, ImageError> {
        let state = SharedState::new();
        let ctx = build_context(pool.clone(), state.clone(), &config.settings)?;

        Ok(Self { pool, state, ctx })
    }

    pub fn context_for(&self, config: &Config) -> Result<DefaultContext, ImageError> {
        build_context(self.pool.clone(), self.state.clone(), &config.settings)
    }

    pub fn ctx(&self) -> &DefaultContext {
        &self.ctx
    }

    pub fn run(&self, config: Config, gpu: Gpu, open_setup: bool) -> iced::Result {
        let pool = self.pool.clone();
        let state = self.state.clone();
        let rebuild = move |config: &Config| {
            build_context(pool.clone(), state.clone(), &config.settings)
                .map_err(|err| err.to_string())
        };

        ui::run(self.ctx.clone(), rebuild, config, gpu, open_setup)
    }
}

fn build_context(
    pool: SqlitePool,
    state: SharedState,
    settings: &Settings,
) -> Result<DefaultContext, ImageError> {
    let chat_model = settings.models.chat.clone();
    let memory_model = settings.models.memory.clone();
    let image_model = settings.models.image.clone();

    // the narrator shares the chat text endpoint/config.
    let distiller = MemoryHandler::new(memory_model.clone());
    let closer = ActClose::new(memory_model);
    let narrator_handler = NarratorHandler::new(chat_model.clone());
    let chat_handler = ChatHandler::new(chat_model);
    let image_handler = ComfyImage::new(image_model.clone())?;

    let director = Director::new(settings.director.clone());

    let adventures = AdventureManager::new(state.clone(), pool.clone());
    let characters = CharacterManager::new(
        state.clone(),
        pool.clone(),
        chat_handler.clone(),
        image_handler,
        CharacterGallery::new(image_model.output_dir),
    );
    let memory_manager = MemoryManager::new(
        state.clone(),
        pool.clone(),
        distiller,
        closer,
        settings.memory.clone(),
    );
    let messages = MessageManager::new(
        state,
        pool,
        chat_handler,
        narrator_handler,
        director,
        memory_manager,
    );

    Ok(AppContext {
        adventure_manager: adventures,
        character_manager: characters,
        message_manager: messages,
    })
}
