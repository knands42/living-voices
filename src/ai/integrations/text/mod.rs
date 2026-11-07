pub mod chat_handler;
pub mod memory_handler;
pub mod narrator_handler;
pub mod open_api_client;

use crate::config::TextModel;

/// The `keep_alive` value to send with a request.
pub(crate) fn keep_alive(config: &TextModel) -> Option<&str> {
    let value = config.keep_alive.trim();
    (!value.is_empty()).then_some(value)
}
