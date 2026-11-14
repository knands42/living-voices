pub mod chat;
pub mod main_hub;
pub mod setup;

use std::sync::Arc;

use crate::config::Config;
use crate::provision::RunningEngines;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    MainHub,
    Chat,
    Setup,
}

pub enum Event {
    None,
    Show(Page),
    EnginesReady {
        engines: Arc<RunningEngines>,
        config: Arc<Config>,
    },
}
