pub mod chat;
pub mod image;
pub mod memory;
pub mod narrator;

use crate::config::TextModel;

/// The `keep_alive` value to send with a request.
fn keep_alive(config: &TextModel) -> Option<&str> {
    let value = config.keep_alive.trim();
    (!value.is_empty()).then_some(value)
}
