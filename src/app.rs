use std::error;

use crate::backend::application::{state::SharedState, world_manager::WorldManagerError};

pub struct App {
    state: SharedState,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: SharedState::new(),
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        todo!()
    }
}

impl App {
    pub fn seed_demo(&self) -> Result<(), WorldManagerError> {
        todo!()
    }
}
