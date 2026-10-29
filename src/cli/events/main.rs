use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::ai::{ActClose, MemoryUpdate};
use crate::app::AppContext;
use crate::cli::events::event_handler::Flow;
use crate::cli::state::{CliState, Page};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::runtime::Handle;

pub fn handle_main(
    key: KeyEvent,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate, ActClose>,
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
