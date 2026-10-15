use thiserror::Error;

use crate::backend::{
    application::state::{SharedState, StateError},
    domain::character::Character,
    persistence::characters::insert_character,
};

#[derive(Error, Debug)]
pub enum CharacterManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
}

#[derive(Clone, Debug)]
pub struct CharacterManager {
    state: SharedState,
}

impl CharacterManager {
    pub fn new(state: SharedState) -> Self {
        CharacterManager { state }
    }

    pub fn create_character(&self, character: Character) -> Result<(), CharacterManagerError> {
        {
            let mut guard = self.state.write()?;
            guard.set_character(character.clone());
        }
        Ok(())
    }
}
