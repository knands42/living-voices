use crate::{backend::application::{adventure_manager::{AdventureManager, AdventureManagerError}, state::SharedState}, ui};

pub struct App {
    state: SharedState,
    adventure_manager: AdventureManager
}

impl App {
    pub fn new() -> Self {
        let state = SharedState::new();
        let adventure_manager = AdventureManager::new(state.clone());
        Self {
            state,
            adventure_manager
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.state.clone())
    }
}

impl App {
    pub fn seed_demo(&self) -> Result<(), AdventureManagerError> {
        todo!()
    }
}
