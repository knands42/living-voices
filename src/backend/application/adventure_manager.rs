use thiserror::Error;

use crate::backend::{
    application::state::{SharedState, StateError},
    domain::adventure::Adventure,
};

#[derive(Error, Debug)]
pub enum AdventureManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
}

pub struct AdventureManager {
    state: SharedState,
}

impl AdventureManager {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }

    pub fn create_adventure(
        &mut self,
        title: String,
        description: String,
    ) -> Result<Adventure, AdventureManagerError> {
        let adventure = Adventure::new(title, description);
        {
            let mut guard = self.state.write()?;
            guard.set_adventure(adventure.clone());
        }
        Ok(adventure)
    }

    pub fn state(&self) -> &SharedState {
        &self.state
    }
}
