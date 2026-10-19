use crate::ai::MemoryUpdate;
use crate::ai::jobs::chat::Chat;
use crate::ai::jobs::narrator::Narrator;
use crate::app::AppContext;
use crate::cli::state::{CliState, Page};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::runtime::Handle;

pub enum Flow {
    Continue,
    Quit,
}

pub fn handle_key(
    key: KeyEvent,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate>,
    runtime: &Handle,
) -> Flow {
    // Esc always backs out of a subpage; only Main treats it as quit (via 'q').
    if matches!(key.code, KeyCode::Esc) && !matches!(state.page, Page::Main) {
        state.page = Page::Main;
        return Flow::Continue;
    }

    // Dispatch by current page tag. Each handler owns its own borrow story.
    match state.page {
        Page::Main => handle_main(key, state, ctx, runtime),
        Page::AdventureList => handle_list(key, state),
        Page::Chat { .. } => handle_chat(key, state),
    }
}

fn handle_main(
    key: KeyEvent,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate>,
    runtime: &Handle,
) -> Flow {
    match key.code {
        KeyCode::Char('q') => Flow::Quit,
        KeyCode::Char('c') => {
            state.page = Page::Chat {
                input: String::new(),
            };
            Flow::Continue
        }
        KeyCode::Char('a') => {
            state.adventures = match ctx.adventures.current_adventure() {
                Some(a) => vec![a],
                None => Vec::new(),
            };
            state.page = Page::AdventureList;
            Flow::Continue
        }
        KeyCode::Char('n') => {
            let res = runtime.block_on(ctx.adventures.create_adventure(
                "New Tale".into(),
                String::new(),
                "Narrator".into(),
                "neutral",
            ));
            match res {
                Ok(a) => {
                    state.adventures.push(a);
                    state.status = Some("new tale created".into());
                }
                Err(e) => state.status = Some(format!("{e}")),
            }
            Flow::Continue
        }
        _ => Flow::Continue,
    }
}

fn handle_list(key: KeyEvent, _state: &mut CliState) -> Flow {
    match key.code {
        KeyCode::Char('q') => Flow::Quit,
        _ => Flow::Continue,
    }
}

/// Composer input: type, backspace, enter. No send wiring yet — visual pass.
fn handle_chat(key: KeyEvent, state: &mut CliState) -> Flow {
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
