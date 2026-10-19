use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::theme::{CARD_EDGE, EMBER, GOLD, INK, INK_DIM, MOSS, RUST};

/// A rounded "pill" chip: ` label ` painted on the given tint.
pub fn pill(label: &str, tint: Color) -> Span<'static> {
    // Owning the label keeps callers free to pass `&format!(..)` temporaries;
    // pills are called a handful of times per frame, not in a hot loop.
    Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(INK)
            .bg(tint)
            .add_modifier(Modifier::BOLD),
    )
}

/// A dot that signals a status at a glance. Pure Unicode, one cell.
pub fn status_dot(kind: StatusKind) -> Span<'static> {
    let (ch, color) = match kind {
        StatusKind::Present => ('●', MOSS),
        StatusKind::Away => ('●', GOLD),
        StatusKind::Unavailable => ('●', RUST),
    };
    Span::styled(ch.to_string(), Style::default().fg(color))
}

pub enum StatusKind {
    Present,
    Away,
    Unavailable,
}

pub fn banner() -> Paragraph<'static> {
    const ROWS: [&str; 6] = [
        r"  _     ___ __     __ ___   _   _    ____ ",
        r" | |   |_ _|\ \   / /|_ _| | \ | |  / ___|",
        r" | |    | |  \ \ / /  | |  |  \| | | |  _ ",
        r" | |___ | |   \ V /   | |  | |\  | | |_| |",
        r" |_____|___|   \_/   |___| |_| \_|  \____|",
        r"           ·  V O I C E S  ·              ",
    ];
    let lines: Vec<Line> = ROWS
        .iter()
        .enumerate()
        .map(|(i, row)| {
            // Fade the top of the banner into gold, the bottom into ember.
            let tint = if i < 2 { GOLD } else { EMBER };
            Line::from(Span::styled(
                *row,
                Style::default().fg(tint).add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    Paragraph::new(lines).alignment(Alignment::Center)
}

/// A card: double-bordered block with a titled header. Caller fills body.
pub fn card<'a>(title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CARD_EDGE))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(EMBER).add_modifier(Modifier::BOLD),
        ))
}

/// A thin footer hint strip: `[key] action · [key] action`.
pub fn hints(items: &[(&str, &str)]) -> Paragraph<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(items.len() * 4);
    for (i, (key, label)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", Style::default().fg(INK_DIM)));
        }
        spans.push(Span::styled(
            format!("[{key}]"),
            Style::default().fg(EMBER).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {label}"), Style::default().fg(INK)));
    }
    Paragraph::new(Line::from(spans)).alignment(Alignment::Center)
}
