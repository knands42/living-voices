use log::debug;

use crate::{
    backend::application::{
        adventure_manager::{AdventureManager, AdventureManagerError},
        seeder::Seeder,
        state::SharedState,
    },
    ui,
};

pub struct App {
    state: SharedState,
    adventure_manager: AdventureManager,
    seeder: Seeder,
}

impl App {
    pub fn new() -> Self {
        let state = SharedState::new();
        let adventure_manager = AdventureManager::new(state.clone());
        Self {
            state,
            adventure_manager,
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.state.clone())
    }
}

impl App {
    pub fn run_demo(&self) -> Result<(), slint::PlatformError> {
        Seeder::run().or_else(|e| -> Result<(), _> {
            debug!("Could not seed demo world: {e}");
            Ok(())
        });
        
        self.run()
    }
}
