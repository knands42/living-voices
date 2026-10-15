use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::backend::domain::adventure::Adventure;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("failed to acquire read lock: {0}")]
    ReadLockFailed(String),
    #[error("failed to acquire write lock: {0}")]
    WriteLockFailed(String),
}

#[derive(Debug, Default)]
pub struct AppState {
    pub current_adventure: Option<Adventure>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_adventure: None,
        }
    }

    pub fn set_adventure(&mut self, adventure: Adventure) {
        self.current_adventure = Some(adventure)
    }
}

#[derive(Clone, Debug)]
pub struct SharedState(Arc<RwLock<AppState>>);

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedState {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(AppState::new())))
    }

    pub fn read(&self) -> Result<std::sync::RwLockReadGuard<'_, AppState>, StateError> {
        self.0
            .try_read()
            .map_err(|e| StateError::ReadLockFailed((e.to_string())))
    }

    pub fn write(&self) -> Result<std::sync::RwLockWriteGuard<'_, AppState>, StateError> {
        self.0
            .try_write()
            .map_err(|e| StateError::WriteLockFailed((e.to_string())))
    }
}
