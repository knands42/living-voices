use sqlx::SqlitePool;

use crate::{
    backend::application::{
        adventure_manager::AdventureManager, character_manager::CharacterManager,
        state::SharedState,
    },
    ui,
};

#[derive(Clone)]
pub struct UiContext {
    pub adventures: AdventureManager,
    pub characters: CharacterManager,
}

pub struct App {
    ctx: UiContext,
}

impl App {
    pub fn new(pool: SqlitePool) -> Self {
        let state = SharedState::new();
        let adventures = AdventureManager::new(state.clone(), pool.clone());
        let characters = CharacterManager::new(state, pool);
        Self {
            ctx: UiContext {
                adventures,
                characters,
            },
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
