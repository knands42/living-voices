use crate::ai::MemoryUpdate;
use crate::ai::jobs::chat::Chat;
use crate::ai::jobs::narrator::Narrator;
use crate::app::AppContext;
use crate::cli::state::CliState;

pub fn render(
    frame: &mut ratatui::Frame,
    state: &CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate>,
) {
}
