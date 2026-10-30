use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::ai::{ActClose, MemoryUpdate};
use crate::app::AppContext;
use crate::cli::state::{CliState, Page};
use ratatui::Frame;

mod adventure_list;
mod avatar;
mod chat;
pub mod main_page;
mod theme;
mod widgets;

pub fn draw(
    frame: &mut Frame,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate, ActClose>,
) {
    match &state.page {
        Page::Main => main_page::render(frame, state, ctx),
        Page::AdventureList => adventure_list::render(frame, state, ctx),
        Page::Chat { .. } => chat::render(frame, state, ctx),
    }
}
