use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::ai::MemoryUpdate;
use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::app::AppContext;
use crate::backend::domain::adventure::Adventure;
use crate::cli::draw::theme::{EMBER, GOLD, INK, INK_DIM, MOSS, dim, ink, muted};
use crate::cli::draw::widgets::{banner, card, hints, pill};
use crate::cli::state::CliState;

pub fn render(frame: &mut Frame, state: &CliState, ctx: &AppContext<Chat, Narrator, MemoryUpdate>) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(8), // banner
            Constraint::Length(1), // spacer
            Constraint::Min(10),   // tale card
            Constraint::Length(3), // menu
            Constraint::Length(1), // footer
        ])
        .split(frame.area());

    frame.render_widget(banner(), rows[0]);
    render_tale_card(frame, rows[2], ctx.adventures.current_adventure().as_ref());
    render_menu(frame, rows[3]);
    render_footer(frame, rows[4], state.status.as_deref());
}

fn render_tale_card(frame: &mut Frame, area: Rect, adventure: Option<&Adventure>) {
    let block = card("Current Tale");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(adventure) = adventure else {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No tale is open.",
                Style::default().fg(INK_DIM).add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press [n] to begin a new one.",
                Style::default().fg(INK),
            )),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    };

    let cast_count = adventure.characters.len();
    let narrator = adventure.narrator.name.as_str();

    let header = Line::from(vec![
        Span::styled("“", Style::default().fg(GOLD)),
        Span::styled(
            adventure.title.as_str(),
            Style::default().fg(EMBER).add_modifier(Modifier::BOLD),
        ),
        Span::styled("”", Style::default().fg(GOLD)),
    ]);

    let meta = Line::from(vec![
        pill(&adventure.tone, EMBER),
        Span::raw("  "),
        pill(&format!("cast: {cast_count}"), MOSS),
        Span::raw("  "),
        pill(&format!("narrator: {narrator}"), GOLD),
    ]);

    let desc_text = if adventure.description.trim().is_empty() {
        "An untold story waits for its first word.".to_string()
    } else {
        adventure.description.clone()
    };

    let body = Paragraph::new(vec![
        Line::from(""),
        header,
        Line::from(""),
        meta,
        Line::from(""),
        Line::from(Span::styled(desc_text, muted())),
    ])
    .alignment(Alignment::Center);

    frame.render_widget(body, inner);
}

fn render_menu(frame: &mut Frame, area: Rect) {
    let menu = hints(&[
        ("c", "enter chat"),
        ("a", "adventures"),
        ("n", "new tale"),
        ("q", "quit"),
    ]);
    frame.render_widget(menu, area);
}

fn render_footer(frame: &mut Frame, area: Rect, status: Option<&str>) {
    let dot = Span::styled("●", Style::default().fg(EMBER));
    let text = status.unwrap_or("ready");
    let line = Line::from(vec![
        dot,
        Span::raw("  "),
        Span::styled(text, dim()),
        Span::raw("   "),
        Span::styled("living voices · tui", ink()),
    ]);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}
