use sqlx::SqlitePool;

use crate::ai::MemoryUpdate;
use crate::{
    backend::application::{
        adventure_manager::AdventureManager, character_manager::CharacterManager,
        memory_manager::MemoryManager, message_manager::MessageManager, state::SharedState,
    },
    config::TextModel,
    ui,
};

#[derive(Clone)]
pub struct AppContext {
    pub adventures: AdventureManager,
    pub characters: CharacterManager,
    pub messages: MessageManager,
}

pub struct App {
    ctx: AppContext,
}

impl App {
    pub fn new(pool: SqlitePool, memory_model: TextModel) -> Self {
        let state = SharedState::new();
        let adventures = AdventureManager::new(state.clone(), pool.clone());
        let characters = CharacterManager::new(state.clone(), pool.clone());
        let messages = MessageManager::new(state.clone(), pool.clone());
        let distiller = MemoryUpdate::new(memory_model.base_url, memory_model.model);
        let _unused = MemoryManager::new(state.clone(), pool.clone(), distiller);
        Self {
            ctx: AppContext {
                adventures,
                characters,
                messages,
            },
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
