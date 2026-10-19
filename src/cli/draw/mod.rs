use crate::ai::MemoryUpdate;
use crate::ai::jobs::chat::Chat;
use crate::ai::jobs::narrator::Narrator;
use crate::app::AppContext;
use crate::cli::state::{CliState, Page};
use ratatui::Frame;

mod adventure_list;
mod chat;
pub mod main_page;

pub fn draw(frame: &mut Frame, state: &CliState, ctx: &AppContext<Chat, Narrator, MemoryUpdate>) {
    match &state.page {
        Page::Main => main_page::render(frame, state, ctx),
        Page::AdventureList => adventure_list::render(frame, state, ctx),
        Page::Chat { input } => chat::render(frame, state, ctx),
    }
}
