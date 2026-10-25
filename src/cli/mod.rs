mod draw;
mod events;
pub mod init;
mod state;

use std::io::Stdout;

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

type Tui = Terminal<CrosstermBackend<Stdout>>;
