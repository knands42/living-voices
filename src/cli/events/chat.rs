use crate::cli::events::event_handler::Flow;
use crate::cli::state::{CliState, Page};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_chat(key: KeyEvent, state: &mut CliState) -> Flow {
    let Page::Chat { input } = &mut state.page else {
        return Flow::Continue;
    };
    match key.code {
        KeyCode::Char('q') if input.is_empty() => Flow::Quit,
        KeyCode::Char(c) => {
            input.push(c);
            Flow::Continue
        }
        KeyCode::Backspace => {
            input.pop();
            Flow::Continue
        }
        KeyCode::Enter => {
            // TODO(send): hand `input` to MessageManager::send_message, then clear.
            input.clear();
            Flow::Continue
        }
        _ => Flow::Continue,
    }
}
