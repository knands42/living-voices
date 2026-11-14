use iced::{Size, mouse};

const SCALE_FROM_WIDTH: f32 = 1440.0;
const SCALE_TO_WIDTH: f32 = 1920.0;
const MAX_SCALE: f32 = 1.15;

pub const DEFAULT_WINDOW: Size = Size::new(1360.0, 900.0);

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
