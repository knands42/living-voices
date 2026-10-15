use crate::{
    backend::application::{adventure_manager::AdventureManager, state::SharedState},
    ui,
};

#[derive(Clone)]
pub struct UiContext {
    pub adventures: AdventureManager
}

pub struct App {
    ctx: UiContext,
}

impl App {
    pub fn new() -> Self {
        let state = SharedState::new();
        let adventures = AdventureManager::new(state);
        Self {
            ctx: UiContext { adventures },
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
