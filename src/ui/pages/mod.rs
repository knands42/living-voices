pub(crate) mod builder;
pub(crate) mod chat;
pub(crate) mod main_hub;
pub(crate) mod setup;

use uuid::Uuid;

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
}
