pub(crate) mod builder;
pub(crate) mod chat;
pub(crate) mod main_hub;
pub(crate) mod setup;

use std::sync::Arc;

use uuid::Uuid;

use crate::config::Config;
use crate::provision::RunningEngines;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    MainHub,
    Builder,
    Chat,
    Setup,
}

pub enum Event {
    None,
    Show(Page),
    OpenBuilder(Uuid),
    OpenChat(Uuid),
    EnginesReady {
        engines: Arc<RunningEngines>,
        config: Arc<Config>,
    },
}
