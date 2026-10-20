use ratatui::style::{Color, Modifier, Style};

pub const INK: Color = Color::Rgb(0xf2, 0xe8, 0xd0);
pub const INK_DIM: Color = Color::Rgb(0xc9, 0xbd, 0xa2);
pub const INK_MUTED: Color = Color::Rgb(0x8a, 0x7f, 0x68);

pub const EMBER: Color = Color::Rgb(0xd9, 0x6a, 0x3a); // accents, active

pub const EMBER_SOFT: Color = Color::Rgb(0xb0, 0x55, 0x2e);
pub const GOLD: Color = Color::Rgb(0xe8, 0xb4, 0x4a);
pub const GOLD_FADED: Color = Color::Rgb(97, 78, 20);
pub const MOSS: Color = Color::Rgb(0x7a, 0x9a, 0x4e);
pub const SKY: Color = Color::Rgb(0x6c, 0x9d, 0xc6);
pub const PLUM: Color = Color::Rgb(0xa0, 0x6c, 0xa8);
pub const RUST: Color = Color::Rgb(0xa8, 0x4a, 0x3c);
pub const CARD_EDGE: Color = Color::Rgb(0x6b, 0x55, 0x3c);

pub const SPEAKER_PALETTE: [Color; 7] = [EMBER, GOLD, MOSS, SKY, PLUM, RUST, GOLD_FADED];

/// Deterministic color for a speaker name — same name, same hue every frame.
pub fn speaker_color(name: &str) -> Color {
    let hash = name
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(131).wrapping_add(b as u32));
    SPEAKER_PALETTE[(hash as usize) % SPEAKER_PALETTE.len()]
}

#[allow(dead_code)]
pub fn title_style() -> Style {
    Style::default().fg(EMBER).add_modifier(Modifier::BOLD)
}

pub fn ink() -> Style {
    Style::default().fg(INK)
}

pub fn dim() -> Style {
    Style::default().fg(INK_DIM)
}

pub fn muted() -> Style {
    Style::default()
        .fg(INK_MUTED)
        .add_modifier(Modifier::ITALIC)
}
