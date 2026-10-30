use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::ai::{ActClose, MemoryUpdate};
use crate::app::{App, AppContext};
use crate::cli::events::chat::handle_mouse;
use crate::cli::events::event_handler::{Flow, handle_key};
use crate::cli::state::CliState;
use crate::cli::{Tui, draw};
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{event, execute};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;
use tokio::runtime::Handle;

pub fn run(app: &App<Chat, Narrator, MemoryUpdate, ActClose>) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut state = CliState::default();
    let runtime = tokio::runtime::Handle::current();
    let result = event_loop(&mut terminal, &mut state, &app.ctx(), &runtime);
    restore_terminal(&mut terminal)?;
    result
}

fn event_loop(
    terminal: &mut Tui,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate, ActClose>,
    runtime: &Handle,
) -> io::Result<()> {
    loop {
        drain_replies(state);

        terminal.draw(|f| draw::draw(f, state, ctx))?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    if matches!(handle_key(key, state, ctx, runtime), Flow::Quit) {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => handle_mouse(mouse, state, ctx),
                _ => {}
            }
        }
    }
}

/// Move any replies produced by background send tasks into the chat.
fn drain_replies(state: &mut CliState) {
    while let Ok(event) = state.reply_rx.try_recv() {
        match event {
            crate::cli::state::ChatEvent::Reply(message) => {
                state.messages.push(message);
                state.status = None;
            }
            crate::cli::state::ChatEvent::Failed(reason) => {
                state.status = Some(format!("reply failed: {reason}"));
            }
        }
    }
}

fn setup_terminal() -> io::Result<Tui> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    if supports_keyboard_enhancement() {
        let _ = execute!(
            stdout,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        );
    }

    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut Tui) -> io::Result<()> {
    if supports_keyboard_enhancement() {
        let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    }
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn supports_keyboard_enhancement() -> bool {
    crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false)
}
