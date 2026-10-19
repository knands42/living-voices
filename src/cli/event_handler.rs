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
    match (&state.page, key.code) {
        (_, KeyCode::Char('q') | KeyCode::Esc) => Flow::Quit,

        (Page::Main, KeyCode::Char('a')) => {
            state.adventures = match ctx.adventures.current_adventure() {
                Some(a) => vec![a],
                None => Vec::new(),
            };
            state.page = Page::AdventureList;
            Flow::Continue
        }

        (Page::Main, KeyCode::Char('n')) => {
            let res = runtime.block_on(ctx.adventures.create_adventure(
                "New Tale".into(),
                "".into(),
                "Narrator".into(),
                "neutral",
            ));
            match res {
                Ok(a) => state.adventures.push(a),
                Err(e) => state.status = Some(format!("{e}")),
            }
            Flow::Continue
        }

        _ => Flow::Continue,
    }
}
