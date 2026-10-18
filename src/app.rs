use sqlx::SqlitePool;

use crate::ai::MemoryUpdate;
use crate::ai::jobs::chat::Chat;
use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::ChatHandler;
use crate::backend::application::port::memory_port::MemoryDistiller;
use crate::{
    backend::application::{
        adventure_manager::AdventureManager, character_manager::CharacterManager,
        memory_manager::MemoryManager, message_manager::MessageManager, state::SharedState,
    },
    config::{MemorySettings, TextModel},
    ui,
};

#[derive(Clone)]
pub struct AppContext<C: ChatHandler + Clone, M: MemoryDistiller + Clone + 'static> {
    pub adventures: AdventureManager,
    pub characters: CharacterManager,
    pub messages: MessageManager<C, M>,
}

pub struct App<C: ChatHandler + Clone, M: MemoryDistiller + Clone + 'static> {
    ctx: AppContext<C, M>,
}

impl App<Chat, MemoryUpdate> {
    pub fn new(
        pool: SqlitePool,
        memory_model: TextModel,
        chat_model: TextModel,
        memory_settings: MemorySettings,
    ) -> Self {
        // state
        let state = SharedState::new();

        // ai modules
        let distiller = MemoryUpdate::new(memory_model);
        let chat_handler = Chat::new(chat_model);

        // application
        let director = Director::new();

        let adventure_manager = AdventureManager::new(state.clone(), pool.clone());
        let character_manager = CharacterManager::new(state.clone(), pool.clone());
        let memory_manager = MemoryManager::new(state.clone(), pool.clone(), distiller);
        let message_manager = MessageManager::new(
            state.clone(),
            pool.clone(),
            chat_handler,
            director,
            memory_manager,
            memory_settings,
        );

        Self {
            ctx: AppContext {
                adventures: adventure_manager,
                characters: character_manager,
                messages: message_manager,
            },
        }
    }
}

impl<C: ChatHandler + Clone + 'static, M: MemoryDistiller + Clone + 'static> App<C, M> {
    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
