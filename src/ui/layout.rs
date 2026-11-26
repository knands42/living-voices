use iced::{Size, mouse};

const SCALE_FROM_WIDTH: f32 = 1440.0;
const SCALE_TO_WIDTH: f32 = 1920.0;
const MAX_SCALE: f32 = 1.15;

pub const DEFAULT_WINDOW: Size = Size::new(1360.0, 900.0);

pub const CHAT_ACT_HIDDEN_BELOW: f32 = 1180.0;
pub const CHAT_FACTS_HIDDEN_BELOW: f32 = 1300.0;
pub const CHAT_CAST_HIDDEN_BELOW: f32 = 900.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rail {
    Cast,
    Facts,
}

impl Rail {
    pub const fn label(self) -> &'static str {
        match self {
            Rail::Cast => "Cast",
            Rail::Facts => "Facts",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Docked,
    Drawer,
}

#[derive(Debug, Clone, Copy)]
pub struct ChatLayout {
    pub act: bool,
    pub cast: Placement,
    pub facts: Placement,
}

pub fn chat_layout(window_width: f32) -> ChatLayout {
    ChatLayout {
        act: window_width >= CHAT_ACT_HIDDEN_BELOW,
        cast: placement(window_width, CHAT_CAST_HIDDEN_BELOW),
        facts: placement(window_width, CHAT_FACTS_HIDDEN_BELOW),
    }
}

fn placement(window_width: f32, docked_above: f32) -> Placement {
    if window_width >= docked_above {
        Placement::Docked
    } else {
        Placement::Drawer
    }
}

pub fn scale_for(window_width: f32) -> f32 {
    let progress =
        ((window_width - SCALE_FROM_WIDTH) / (SCALE_TO_WIDTH - SCALE_FROM_WIDTH)).clamp(0.0, 1.0);

    1.0 + (MAX_SCALE - 1.0) * progress
}

pub const MIN_ZOOM: f32 = 0.8;
pub const MAX_ZOOM: f32 = 1.2;
const ZOOM_PER_LINE: f32 = 0.05;
const ZOOM_PER_PIXEL: f32 = 0.002;

pub fn zoomed(zoom: f32, delta: mouse::ScrollDelta) -> f32 {
    let step = match delta {
        mouse::ScrollDelta::Lines { y, .. } => y * ZOOM_PER_LINE,
        mouse::ScrollDelta::Pixels { y, .. } => y * ZOOM_PER_PIXEL,
    };

    (zoom + step).clamp(MIN_ZOOM, MAX_ZOOM)
}
