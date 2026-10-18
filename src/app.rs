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
    config::TextModel,
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
    /// Composition root: the only place that names the concrete `Chat`/`MemoryUpdate` impls.
    pub fn new(pool: SqlitePool, memory_model: TextModel, chat_model: TextModel) -> Self {
        // state
        let state = SharedState::new();

        // ai modules
        let distiller = MemoryUpdate::new(memory_model.base_url, memory_model.model);
        let chat_handler = Chat::new(chat_model.base_url, chat_model.model);

        // application
        let director = Director::new();

        let adventures = AdventureManager::new(state.clone(), pool.clone());
        let characters = CharacterManager::new(state.clone(), pool.clone());
        let memory = MemoryManager::new(state.clone(), pool.clone(), distiller);
        let messages =
            MessageManager::new(state.clone(), pool.clone(), chat_handler, director, memory);

        Self {
            ctx: AppContext {
                adventures,
                characters,
                messages,
            },
        }
    }
}

impl<C: ChatHandler + Clone + 'static, M: MemoryDistiller + Clone + 'static> App<C, M> {
    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
