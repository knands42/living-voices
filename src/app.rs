use sqlx::SqlitePool;

use crate::ai::MemoryUpdate;
use crate::ai::jobs::chat::Chat;
use crate::backend::application::port::chat_port::ChatHandler;
use crate::{
    backend::application::{
        adventure_manager::AdventureManager, character_manager::CharacterManager,
        memory_manager::MemoryManager, message_manager::MessageManager, state::SharedState,
    },
    config::TextModel,
    ui,
};

#[derive(Clone)]
pub struct AppContext<C: ChatHandler + Clone> {
    pub adventures: AdventureManager,
    pub characters: CharacterManager,
    pub messages: MessageManager<C>,
}

pub struct App<C: ChatHandler + Clone> {
    ctx: AppContext<C>,
}

impl App<Chat> {
    /// Composition root: the one place allowed to name the concrete `Chat`
    /// (and `MemoryUpdate`) implementations and wire them in.
    pub fn new(pool: SqlitePool, memory_model: TextModel, chat_model: TextModel) -> Self {
        // state
        let state = SharedState::new();

        // ai modules
        let distiller = MemoryUpdate::new(memory_model.base_url, memory_model.model);
        let chat_handler = Chat::new(chat_model.base_url, chat_model.model);

        // application managers
        let adventures = AdventureManager::new(state.clone(), pool.clone());
        let characters = CharacterManager::new(state.clone(), pool.clone());
        let messages = MessageManager::new(state.clone(), pool.clone(), chat_handler);
        let _unused = MemoryManager::new(state.clone(), pool.clone(), distiller);

        Self {
            ctx: AppContext {
                adventures,
                characters,
                messages,
            },
        }
    }
}

impl<C: ChatHandler + Clone + 'static> App<C> {
    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
