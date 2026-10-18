use slint::{Color, ModelRc, SharedString, VecModel};

use crate::{
    backend::domain::{adventure::Adventure, character::Character},
    ui::{ChatCharacter, RootWindow},
};

/// The one place to wire backend data into the root window, read by both pages.
pub fn fill_root_window_inputs(ui: &RootWindow, session: Option<&Adventure>) {
    let Some(adventure) = session else {
        return;
    };

    // Build the chat sidebar cast from the adventure's real characters.
    let cast: Vec<ChatCharacter> = adventure
        .characters
        .iter()
        .enumerate()
        .map(|(i, character)| to_chat_character(character, i == 0))
        .collect();

    // Default the "address" target to the first cast member, so Say/Whisper
    // work out of the box.
    if let Some(first) = cast.first() {
        ui.set_whisper_target(first.clone());
    }

    ui.set_characters(ModelRc::from(std::rc::Rc::new(VecModel::from(cast))));
}

/// Map a domain character to the sidebar shape. The avatar glyph is the first
/// letter of the name; the tint is derived from the name so it stays stable.
fn to_chat_character(character: &Character, active: bool) -> ChatCharacter {
    let glyph: SharedString = character
        .name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default()
        .into();

    ChatCharacter {
        name: character.name.as_str().into(),
        role: character.status.as_str().into(),
        glyph,
        hue: hue_from_name(&character.name),
        active,
        location: SharedString::new(),
        level: SharedString::new(),
        status: SharedString::new(),
    }
}

/// A stable, muted avatar tint derived from the character's name.
fn hue_from_name(name: &str) -> Color {
    let hash = name
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    // Keep each channel in a dim, earthy range to match the parchment theme.
    let r = 0x2a + (hash & 0x1f) as u8;
    let g = 0x24 + ((hash >> 5) & 0x1f) as u8;
    let b = 0x1f + ((hash >> 10) & 0x1f) as u8;
    Color::from_rgb_u8(r, g, b)
}
