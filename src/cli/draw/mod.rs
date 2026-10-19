use crate::cli::state::{CliState, Page};
use ratatui::Frame;

mod adventure_list;
mod chat;
pub mod main_page;

pub fn draw(frame: &mut Frame, state: &CliState) {
    match &state.page {
        Page::Main => main_page::render(frame, state),
        Page::AdventureList => adventure_list::render(frame, state),
        Page::Chat { input } => chat::render(frame, state),
    }
}
