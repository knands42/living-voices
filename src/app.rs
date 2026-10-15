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
    pub fn new() -> Self {
        let state = SharedState::new();
        let adventures = AdventureManager::new(state.clone());
        let characters = CharacterManager::new(state);
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
