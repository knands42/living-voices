use sqlx::SqlitePool;

use crate::{
    backend::application::{
        adventure_manager::AdventureManager, character_manager::CharacterManager,
        memory_manager::MemoryManager, message_manager::MessageManager, state::SharedState,
    },
    ui,
};

#[derive(Clone)]
pub struct AppContext {
    pub adventures: AdventureManager,
    pub characters: CharacterManager,
    pub messages: MessageManager,
    pub memory: MemoryManager,
}

pub struct App {
    ctx: AppContext,
}

impl App {
    pub fn new(pool: SqlitePool) -> Self {
        let state = SharedState::new();
        let adventures = AdventureManager::new(state.clone(), pool.clone());
        let characters = CharacterManager::new(state.clone(), pool.clone());
        let messages = MessageManager::new(state.clone(), pool.clone());
        let memory = MemoryManager::new(state.clone(), pool.clone());
        Self {
            ctx: AppContext {
                adventures,
                characters,
                messages,
                memory,
            },
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
