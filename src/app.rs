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
    config::{DirectorSettings, ImageModel, MemorySettings, TextModel},
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
    pub adventures: AdventureManager,
    pub characters: CharacterManager<P, I>,
    pub messages: MessageManager<C, N, M, A>,
}

pub type DefaultMessageManager =
    MessageManager<ChatHandler, NarratorHandler, MemoryHandler, ActClose>;

pub type DefaultContext =
    AppContext<ChatHandler, NarratorHandler, MemoryHandler, ActClose, ChatHandler, ComfyImage>;

pub struct App {
    ctx: DefaultContext,
}

impl App {
    pub fn new(
        pool: SqlitePool,
        memory_model: TextModel,
        chat_model: TextModel,
        image_model: ImageModel,
        memory_settings: MemorySettings,
        director_settings: DirectorSettings,
    ) -> Result<Self, ImageError> {
        // state
        let state = SharedState::new();

        // ai modules — the narrator shares the chat text endpoint/config.
        let distiller = MemoryHandler::new(memory_model.clone());
        let closer = ActClose::new(memory_model);
        let narrator_handler = NarratorHandler::new(chat_model.clone());
        let chat_handler = ChatHandler::new(chat_model);
        let image_handler = ComfyImage::new(image_model.clone())?;

        // application
        let director = Director::new(director_settings);

        let adventure_manager = AdventureManager::new(state.clone(), pool.clone());
        let character_manager = CharacterManager::new(
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
            memory_settings,
        );
        let message_manager = MessageManager::new(
            state.clone(),
            pool.clone(),
            chat_handler,
            narrator_handler,
            director,
            memory_manager,
        );

        Ok(Self {
            ctx: AppContext {
                adventures: adventure_manager,
                characters: character_manager,
                messages: message_manager,
            },
        })
    }

    pub fn ctx(&self) -> &DefaultContext {
        &self.ctx
    }

    #[cfg(debug_assertions)]
    pub async fn load_adventure(
        &self,
        adventure: crate::backend::domain::adventure::Adventure,
    ) -> Result<(), crate::backend::application::adventure_manager::AdventureManagerError> {
        self.ctx.adventures.start_adventure(adventure.id).await?;

        Ok(())
    }

    pub fn run(&self) -> iced::Result {
        ui::run()
    }
}
