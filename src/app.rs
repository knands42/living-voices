use sqlx::SqlitePool;

use crate::ai::{ActClose, ChatHandler, ComfyImage, MemoryHandler, NarratorHandler};
use crate::backend::application::character_gallery::CharacterGallery;
use crate::backend::application::director::Director;
use crate::backend::application::managers::message::message_manager::MessageManager;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::image_port::{ImageError, ImagePort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::{
    backend::application::{
        adventure_manager::AdventureManager, character_manager::CharacterManager,
        memory_manager::MemoryManager, state::SharedState,
    },
    config::ConfigHandle,
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
    pub fn new(pool: SqlitePool, config: ConfigHandle) -> Result<Self, ImageError> {
        let state = SharedState::new();
        let ctx = build_context(pool.clone(), state.clone(), config)?;

        Ok(Self { pool, state, ctx })
    }

    pub fn context_for(&self, config: ConfigHandle) -> Result<DefaultContext, ImageError> {
        build_context(self.pool.clone(), self.state.clone(), config)
    }

    pub fn ctx(&self) -> &DefaultContext {
        &self.ctx
    }

    pub fn run(&self, config: ConfigHandle, gpu: Gpu, open_setup: bool) -> iced::Result {
        ui::run(self.ctx.clone(), config, gpu, open_setup)
    }
}

fn build_context(
    pool: SqlitePool,
    state: SharedState,
    config: ConfigHandle,
) -> Result<DefaultContext, ImageError> {
    let settings = config.settings();
    let image_model = settings.models.image.clone();

    // the narrator shares the chat text endpoint/config.
    let distiller = MemoryHandler::new(config.clone());
    let closer = ActClose::new(config.clone());
    let narrator_handler = NarratorHandler::new(config.clone());
    let chat_handler = ChatHandler::new(config);
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
