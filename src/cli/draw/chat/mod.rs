use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::ai::{ActClose, MemoryUpdate};
use crate::app::AppContext;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::{Character, CharacterStatus};
use crate::backend::domain::message::Message;
use crate::cli::draw::avatar::{Avatar, AvatarKind};
use crate::cli::draw::theme::{
    CARD_EDGE, EMBER, GOLD, GOLD_FADED, INK, PANEL, VOID, dim, fill_bg, ink, muted, scatter_runes,
};
use crate::cli::draw::widgets::{StatusKind, card, hints, pill, status_dot};
use crate::cli::state::{CliState, Page};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

mod transcript;

pub fn render(
    frame: &mut Frame,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate, ActClose>,
) {
    let (input, cursor) = match &state.page {
        Page::Chat { input, cursor } => (input.as_str(), *cursor),
        _ => ("", 0),
    };
    let adventure = ctx.adventures.current_adventure();
    let adventure_ref = adventure.as_ref();

    // Paint the void behind everything
    let full = frame.area();
    let buf = frame.buffer_mut();
    fill_bg(full, buf, VOID);
    scatter_runes(full, buf, 23);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),    // body (3 columns)
            Constraint::Length(3), // composer
            Constraint::Length(1), // hints
        ])
        .split(frame.area());

    render_header(frame, rows[0], adventure_ref);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(26), // cast
            Constraint::Min(30),    // chat
            Constraint::Length(28), // lore
        ])
        .split(rows[1]);

    state.cast_row_hitboxes = render_cast(frame, cols[0], adventure_ref);
    render_transcript(frame, cols[1], adventure_ref, &state.messages);
    render_meta(frame, cols[2], adventure_ref);

    render_composer(frame, rows[2], input, cursor, adventure_ref);
    render_hints(frame, rows[3], state.status.as_deref());

    render_character_overlay(frame, full, adventure_ref, state.hovered_character);
}

// ---------- Header ----------

fn render_header(frame: &mut Frame, area: Rect, adventure: Option<&Adventure>) {
    let block = Block::default()
        .borders(ratatui::widgets::Borders::BOTTOM)
        .border_style(Style::default().fg(CARD_EDGE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (title, tone, narrator) = match adventure {
        Some(a) => (a.title.as_str(), a.tone.as_str(), a.narrator.name.as_str()),
        None => ("No tale open", "—", "—"),
    };

    let line = Line::from(vec![
        Span::styled("❈ ", Style::default().fg(GOLD)),
        Span::styled(
            title,
            Style::default()
                .fg(EMBER)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
        ),
        Span::styled(" ❖ ", Style::default().fg(GOLD_FADED)),
        pill(tone, EMBER),
        Span::raw("  "),
        Span::styled("⟡ narrated by ", dim()),
        Span::styled(
            narrator,
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), inner);
}

// ---------- Cast sidebar ----------

/// Renders the cast column and returns one hitbox Rect per rendered row, in the
/// same order as `sorted_cast` — so a mouse hit maps straight to that index.
fn render_cast(frame: &mut Frame, area: Rect, adventure: Option<&Adventure>) -> Vec<Rect> {
    let block = card("Cast");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(adventure) = adventure else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no cast", muted())))
                .alignment(Alignment::Center),
            inner,
        );
        return Vec::new();
    };

    let mut hitboxes = Vec::new();
    let mut row_top = inner.top();

    // Row is 3 lines tall: [avatar+name] [role+status] [spacer]
    for character in sorted_cast(adventure) {
        let row_h: u16 = 3;
        if row_top + row_h > inner.bottom() {
            break;
        }
        let row = Rect {
            x: inner.left(),
            y: row_top,
            width: inner.width,
            height: row_h,
        };
        render_cast_row(frame, row, character);
        hitboxes.push(row);
        row_top += row_h;
    }

    hitboxes
}

/// The cast in display order: the player first, then the rest as stored.
/// Kept as a named helper so the mouse hit-test resolves the same index the
/// renderer drew.
pub fn sorted_cast(adventure: &Adventure) -> Vec<&Character> {
    let mut cast: Vec<&Character> = adventure.characters.iter().collect();
    cast.sort_by_key(|c| !c.is_player);
    cast
}

fn render_cast_row(frame: &mut Frame, area: Rect, character: &Character) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(4), Constraint::Min(1)])
        .split(area);

    // Avatar square (2 cells tall inside the 3-line row).
    let avatar_area = Rect {
        x: cols[0].left() + 1,
        y: cols[0].top(),
        width: 3,
        height: 2,
    };
    let mut avatar = AvatarKind::glyph(&character.name);
    let buf = frame.buffer_mut();
    avatar.render(avatar_area, buf);

    // Right side: name + status dot on line 1, role on line 2.
    let is_player = character.is_player;
    let name_color = if is_player { EMBER } else { INK };
    let name_line = Line::from(vec![
        Span::raw(" "),
        status_dot(to_status_kind(character.status)),
        Span::raw(" "),
        Span::styled(
            character.name.as_str(),
            Style::default().fg(name_color).add_modifier(if is_player {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ]);

    let role_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(character.status.as_str(), muted()),
    ]);
    let text_area = Rect {
        x: cols[1].left(),
        y: cols[1].top(),
        width: cols[1].width.saturating_sub(1),
        height: 2,
    };
    frame.render_widget(Paragraph::new(vec![name_line, role_line]), text_area);
}

fn to_status_kind(status: CharacterStatus) -> StatusKind {
    match status {
        CharacterStatus::Present => StatusKind::Present,
        CharacterStatus::Away => StatusKind::Away,
        CharacterStatus::Unavailable => StatusKind::Unavailable,
    }
}

// ---------- Transcript (center) ----------

fn render_transcript(
    frame: &mut Frame,
    area: Rect,
    adventure: Option<&Adventure>,
    messages: &[Message],
) {
    let block = card("Scene");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_bg(inner, frame.buffer_mut(), PANEL);

    if adventure.is_none() {
        scatter_runes(inner, frame.buffer_mut(), 17);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "❧ unseal a tale to begin the scene",
                muted(),
            )))
            .alignment(Alignment::Center),
            inner,
        );
        return;
    }

    if messages.is_empty() {
        scatter_runes(inner, frame.buffer_mut(), 17);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "❧ the scene lies still — speak, and it stirs",
                muted(),
            )))
            .alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let lines: Vec<Line> = messages.iter().map(transcript::message_line).collect();

    // Keep the latest lines in view: scroll so the tail sits at the bottom.
    let total = lines.len() as u16;
    let visible = inner.height;
    let scroll = total.saturating_sub(visible);

    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .alignment(Alignment::Left);
    frame.render_widget(p, inner);
}

// ---------- Meta rail (right) ----------

fn render_meta(frame: &mut Frame, area: Rect, adventure: Option<&Adventure>) {
    let block = card("Lore");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(adventure) = adventure else {
        return;
    };

    let tags_line: Line = if adventure.tags.is_empty() {
        Line::from(Span::styled("no tags", muted()))
    } else {
        let mut spans: Vec<Span> = Vec::with_capacity(adventure.tags.len() * 2);
        for (i, tag) in adventure.tags.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(pill(tag, GOLD_FADED));
        }
        Line::from(spans)
    };

    let genre = adventure.genre.as_deref().unwrap_or("—");

    let body = Paragraph::new(vec![
        Line::from(Span::styled("Genre", dim())),
        Line::from(Span::styled(genre, ink().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Tone", dim())),
        Line::from(Span::styled(adventure.tone.as_str(), ink())),
        Line::from(""),
        Line::from(Span::styled("Tags", dim())),
        tags_line,
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(body, inner);
}

// ---------- Composer ----------

fn render_composer(
    frame: &mut Frame,
    area: Rect,
    input: &str,
    cursor: usize,
    adventure: Option<&Adventure>,
) {
    let block = card("Say");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let target = adventure
        .and_then(|a| a.characters.first())
        .map(|c| c.name.as_str())
        .unwrap_or("—");

    let caret = Span::styled(
        "▎",
        Style::default()
            .fg(EMBER)
            .add_modifier(Modifier::SLOW_BLINK),
    );

    // Split the input at the cursor (a char index) so the caret sits where the
    // next edit lands, not always at the end.
    let chars: Vec<char> = input.chars().collect();
    let split = cursor.min(chars.len());
    let before: String = chars[..split].iter().collect();
    let after: String = chars[split..].iter().collect();

    let line = Line::from(vec![
        Span::styled(
            "❯ ",
            Style::default().fg(EMBER).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            target,
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ⟫ ", dim()),
        Span::styled(before, ink()),
        caret,
        Span::styled(after, ink()),
    ]);
    frame.render_widget(Paragraph::new(line), inner);
}

// ---------- Character overlay ----------

/// When a cast member is hovered, draw its description in a small centred panel
/// over the scene. `Clear` punches a hole first so the panel isn't see-through.
fn render_character_overlay(
    frame: &mut Frame,
    area: Rect,
    adventure: Option<&Adventure>,
    hovered: Option<usize>,
) {
    let (Some(adventure), Some(index)) = (adventure, hovered) else {
        return;
    };
    let cast = sorted_cast(adventure);
    let Some(character) = cast.get(index) else {
        return;
    };

    let panel = centered_rect(area, 48, 40);
    frame.render_widget(ratatui::widgets::Clear, panel);

    let block = card(character.name.as_str());
    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    let description = if character.description.trim().is_empty() {
        "no description yet"
    } else {
        character.description.as_str()
    };

    let mut lines = vec![
        Line::from(vec![
            status_dot(to_status_kind(character.status)),
            Span::raw(" "),
            Span::styled(character.status.as_str(), muted()),
        ]),
        Line::from(Span::styled("─────────", Style::default().fg(CARD_EDGE))),
        Line::from(Span::styled(description.to_string(), ink())),
    ];

    if !character.traits.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("⟡ Traits", dim())));
        lines.push(Line::from(Span::styled(
            character.traits.join(", "),
            ink().add_modifier(Modifier::ITALIC),
        )));
    }

    let body = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(body, inner);
}

/// A Rect centred in `area`, sized to the given percentage of width/height.
fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

// ---------- Hints ----------

fn render_hints(frame: &mut Frame, area: Rect, status: Option<&str>) {
    if let Some(status) = status {
        let line = Line::from(vec![
            Span::styled("● ", Style::default().fg(EMBER)),
            Span::styled(status.to_string(), ink()),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let bar = hints(&[
        ("enter", "speak"),
        ("shift+enter", "new line"),
        ("esc", "withdraw"),
        ("q", "depart"),
    ]);
    frame.render_widget(bar, area);
}
