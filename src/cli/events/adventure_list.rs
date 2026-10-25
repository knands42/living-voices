use crate::cli::events::event_handler::Flow;
use crate::cli::state::CliState;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_list(key: KeyEvent, _state: &mut CliState) -> Flow {
    match key.code {
        KeyCode::Char('q') => Flow::Quit,
        _ => Flow::Continue,
    }
}
