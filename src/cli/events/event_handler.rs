use crate::ai::MemoryUpdate;
use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::app::AppContext;
use crate::cli::events::adventure_list::handle_list;
use crate::cli::events::chat::handle_chat;
use crate::cli::events::main::handle_main;
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
        Page::Chat { .. } => handle_chat(key, state, ctx, runtime),
    }
}
