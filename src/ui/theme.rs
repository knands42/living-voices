use std::f32::consts::FRAC_PI_2;

use iced::gradient::Linear;
use iced::widget::{button, container, progress_bar, text_input};
use iced::{Background, Border, Color, Font, Radians, Shadow, Theme, Vector};


// The palette
pub const BG_DEEPEST: Color = Color::from_rgb8(0x12, 0x10, 0x0e);
pub const BG_LOW: Color = Color::from_rgb8(0x0b, 0x0a, 0x08);
pub const BG_PANEL: Color = Color::from_rgb8(0x1c, 0x18, 0x15);
pub const BG_PANEL_ALT: Color = Color::from_rgb8(0x26, 0x21, 0x1b);
pub const BG_SUNK: Color = Color::from_rgb8(0x0e, 0x0c, 0x0a);
pub const BRONZE: Color = Color::from_rgb8(0xa5, 0x75, 0x4a);
pub const BRONZE_DIM: Color = Color::from_rgb8(0x45, 0x33, 0x26);
pub const GOLD: Color = Color::from_rgb8(0xe3, 0xb5, 0x62);
pub const AMBER: Color = Color::from_rgb8(0xd8, 0x9c, 0x4a);
pub const ROSE: Color = Color::from_rgb8(0xc9, 0x6b, 0x5f);
pub const SEA: Color = Color::from_rgb8(0x5f, 0x9e, 0xa8);
pub const TEXT_LIGHT: Color = Color::from_rgb8(0xed, 0xe4, 0xd6);
pub const TEXT_MUTED: Color = Color::from_rgb8(0x9a, 0x90, 0x82);
pub const TEXT_DIM: Color = Color::from_rgb8(0x6b, 0x63, 0x58);

/// `hsv(hue, saturation, value)`: `hue` in degrees, the other two in `0.0..=1.0`
/// TODO: Is hsv really needed or just a leftover to keep consistent with slint?
pub fn hsv(hue: f32, saturation: f32, value: f32) -> Color {
    let h = hue.rem_euclid(360.0) / 60.0;
    let c = value * saturation;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let m = value - c;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::from_rgb(r + m, g + m, b + m)
}

/// Same colour, new alpha. Every fade in the sketch goes through here.
pub fn alpha(color: Color, a: f32) -> Color {
    Color { a: a.clamp(0.0, 1.0), ..color }
}

/// Blend two colours by `t` (`0.0` = `from`, `1.0` = `to`).
pub fn blend(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

/// A left-to-right linear gradient. iced measures the angle from the top, so
/// `PI / 2` runs along the x axis; see `iced::angle::Radians::to_distance`.
pub fn horizontal(stops: &[(f32, Color)]) -> Linear {
    let mut linear = Linear::new(Radians(FRAC_PI_2));
    for (offset, color) in stops {
        linear = linear.add_stop(*offset, *color);
    }
    linear
}

// --- page and panels ------------------------------------------------------

/// The page itself: a very quiet vertical wash so the dark does not read flat.
pub fn page(_theme: &Theme) -> container::Style {
    container::Style::default().background(vertical(&[
        (0.0, BG_LOW),
        (1.0, Color::from_rgb8(0x17, 0x13, 0x10)),
    ]))
}

/// A vertical gradient, top to bottom. Angle `0.0` runs bottom to top.
fn vertical(stops: &[(f32, Color)]) -> Linear {
    let mut linear = Linear::new(Radians(0.0));
    for (offset, color) in stops {
        linear = linear.add_stop(*offset, *color);
    }
    linear
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(vertical(&[
            (0.0, BG_PANEL),
            (1.0, Color::from_rgb8(0x21, 0x1c, 0x18)),
        ]))
        .border(Border::default().rounded(18.0).width(1.0).color(BRONZE_DIM))
        .shadow(Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.42),
            offset: Vector::new(0.0, 14.0),
            blur_radius: 32.0,
        })
}

/// A quieter inset surface, for sections inside a panel.
pub fn inset(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(alpha(Color::BLACK, 0.22))
        .border(Border::default().rounded(12.0).width(1.0).color(alpha(BRONZE_DIM, 0.7)))
}

/// The dimmed card the player sits in, at the top of the cast list.
pub fn player_card(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(alpha(BRONZE, 0.07))
        .border(Border::default().rounded(12.0).width(1.0).color(alpha(BRONZE, 0.35)))
}

// --- cast ----------------------------------------------------------------

/// A cast row, with the hover highlight sweeping in from the left.
///
/// `sweep` is the animation value: `0.0` untouched, `1.0` fully lit. The
/// gradient's last stop *is* the leading edge of the highlight, which is why
/// this is a gradient and not a background colour.
pub fn cast_row(sweep: f32, selected: bool, hue: f32) -> container::Style {
    let sweep = sweep.clamp(0.0, 1.0);

    if selected {
        let lit = hsv(hue, 0.34, 0.30);
        return container::Style::default()
            .background(horizontal(&[
                (0.0, alpha(lit, 0.55)),
                (0.7, alpha(lit, 0.22)),
                (1.0, alpha(lit, 0.10)),
            ]))
            .border(Border::default().rounded(12.0).width(1.0).color(alpha(hsv(hue, 0.45, 0.72), 0.85)));
    }

    if sweep <= 0.001 {
        return container::Style::default()
            .border(Border::default().rounded(12.0).width(1.0).color(Color::TRANSPARENT));
    }

    // Leading edge at `sweep`, with a soft shoulder behind it.
    let lit = alpha(blend(Color::from_rgb8(0xf2, 0xe8, 0xd8), hsv(hue, 0.30, 0.90), 0.45), 0.10);
    let shoulder = alpha(hsv(hue, 0.40, 0.85), 0.16);

    let mut g = horizontal(&[(0.0, lit)]);
    if sweep > 0.12 {
        g = g.add_stop(sweep - 0.12, lit);
    }
    g = g.add_stop(sweep, shoulder);
    g = g.add_stop((sweep + 0.001).min(1.0), Color::TRANSPARENT);

    container::Style::default()
        .background(g)
        .border(Border::default().rounded(12.0).width(1.0).color(alpha(hsv(hue, 0.40, 0.70), 0.30 * sweep)))
}

pub fn portrait(hue: f32, glow: f32) -> container::Style {
    container::Style::default()
        .background(hsv(hue, 0.32, 0.24))
        .border(Border::default().rounded(999.0).width(1.5).color(alpha(hsv(hue, 0.48, 0.72), 0.55 + 0.45 * glow)))
}

/// The thin accent bar on the left of scene cards.
pub fn accent(color: Color, a: f32) -> container::Style {
    container::Style::default()
        .background(alpha(color, a))
        .border(Border::default().rounded(999.0))
}

// --- speech ---------------------------------------------------------------

pub fn bubble(hue: f32, is_player: bool, a: f32) -> container::Style {
    if is_player {
        container::Style::default()
            .background(horizontal(&[
                (0.0, alpha(hsv(hue, 0.30, 0.26), a)),
                (1.0, alpha(hsv(hue, 0.26, 0.18), a)),
            ]))
            .border(Border::default().rounded(16.0).width(1.0).color(alpha(hsv(hue, 0.44, 0.50), a)))
    } else {
        container::Style::default()
            .background(alpha(BG_PANEL_ALT, a))
            .border(Border::default().rounded(16.0).width(1.0).color(alpha(BRONZE_DIM, a)))
    }
}

pub fn narrator(a: f32) -> container::Style {
    container::Style::default()
        .background(horizontal(&[
            (0.0, alpha(GOLD, 0.02 * a)),
            (0.5, alpha(GOLD, 0.07 * a)),
            (1.0, alpha(GOLD, 0.02 * a)),
        ]))
        .border(Border::default().rounded(14.0).width(1.0).color(alpha(BRONZE, 0.30 * a)))
}

/// The inline dice result. Gold when it lands, rose when it costs you.
pub fn roll_chip(color: Color, a: f32) -> container::Style {
    container::Style::default()
        .background(alpha(color, 0.08 * a))
        .border(Border::default().rounded(999.0).width(1.0).color(alpha(color, 0.45 * a)))
}

// --- interactive ----------------------------------------------------------

pub fn back_button(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: Some(Background::Color(if hovered {
            BG_PANEL_ALT
        } else {
            alpha(BG_PANEL, 0.85)
        })),
        text_color: TEXT_LIGHT,
        border: Border::default().rounded(999.0).width(1.0).color(alpha(BRONZE, if hovered { 0.9 } else { 0.5 })),
        ..button::Style::default()
    }
}

pub fn send_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered => (Some(Background::Color(alpha(GOLD, 0.88))), BG_DEEPEST),
        button::Status::Pressed => (Some(Background::Color(BRONZE)), BG_DEEPEST),
        button::Status::Disabled => (
            Some(Background::Color(alpha(BG_PANEL_ALT, 0.7))),
            alpha(TEXT_DIM, 0.7),
        ),
        button::Status::Active => (Some(Background::Color(GOLD)), BG_DEEPEST),
    };

    button::Style {
        background,
        text_color,
        border: Border::default().rounded(14.0).width(1.0).color(alpha(BRONZE_DIM, 0.9)),
        shadow: Shadow {
            color: alpha(GOLD, 0.18),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        ..button::Style::default()
    }
}

/// The one-tap suggestions under the composer.
pub fn quick_chip(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let active = matches!(status, button::Status::Active);

    button::Style {
        background: Some(Background::Color(if hovered {
            alpha(AMBER, 0.16)
        } else if active {
            alpha(BG_PANEL_ALT, 0.6)
        } else {
            Color::TRANSPARENT
        })),
        text_color: if hovered { GOLD } else { TEXT_MUTED },
        border: Border::default()
            .rounded(999.0)
            .width(1.0)
            .color(if hovered { alpha(GOLD, 0.55) } else { alpha(BRONZE_DIM, 0.9) }),
        ..button::Style::default()
    }
}

pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: Some(Background::Color(alpha(
            Color::WHITE,
            if hovered { 0.10 } else { 0.04 },
        ))),
        text_color: if hovered { TEXT_LIGHT } else { TEXT_MUTED },
        border: Border::default().rounded(999.0).width(1.0).color(alpha(BRONZE_DIM, 0.9)),
        ..button::Style::default()
    }
}

pub fn composer(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });

    text_input::Style {
        background: Background::Color(alpha(BG_SUNK, 0.92)),
        border: Border::default()
            .rounded(14.0)
            .width(1.0)
            .color(if focused { alpha(GOLD, 0.55) } else { BRONZE_DIM }),
        icon: TEXT_MUTED,
        placeholder: TEXT_DIM,
        value: TEXT_LIGHT,
        selection: alpha(BRONZE, 0.85),
    }
}

/// The chips under a dossier's name.
pub fn trait_chip(hue: f32) -> container::Style {
    container::Style::default()
        .background(alpha(hsv(hue, 0.30, 0.55), 0.16))
        .border(Border::default().rounded(999.0).width(1.0).color(alpha(hsv(hue, 0.40, 0.70), 0.35)))
}

// --- bars -----------------------------------------------------------------

/// A small progress bar: bond, thread heat, the tide.
pub fn bar(fill: Color, track: Color) -> impl Fn(&Theme) -> progress_bar::Style {
    move |_theme| progress_bar::Style {
        background: Background::Color(track),
        bar: Background::Color(fill),
        border: Border::default().rounded(999.0),
    }
}

// --- overlays -------------------------------------------------------------

pub fn lightbox(_theme: &Theme) -> container::Style {
    container::Style::default().background(Color::from_rgba(0.03, 0.03, 0.04, 0.92))
}

pub fn lightbox_card(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(BG_PANEL)
        .border(Border::default().rounded(18.0).width(1.0).color(alpha(BRONZE, 0.45)))
        .shadow(Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
            offset: Vector::new(0.0, 24.0),
            blur_radius: 60.0,
        })
}

/// The gallery tile's frame: a hairline that warms on hover.
pub fn tile(hovered: bool, hue: f32) -> container::Style {
    container::Style::default()
        .border(Border::default().rounded(10.0).width(1.0).color(if hovered {
            alpha(hsv(hue, 0.45, 0.85), 0.9)
        } else {
            alpha(BRONZE_DIM, 0.9)
        }))
        .background(BG_SUNK)
}

// --- setup / launcher -----------------------------------------------------
//
// The first screen before the app proper. Same palette as the page, but the
// surfaces are calmer: one panel per question, readings in sunk cells, and a
// tinted callout whose colour carries the verdict.

/// A tinted callout: `color` catches the left edge, then fades across.
pub fn callout(color: Color, a: f32) -> container::Style {
    container::Style::default()
        .background(horizontal(&[
            (0.0, alpha(color, 0.15 * a)),
            (0.55, alpha(color, 0.055 * a)),
            (1.0, alpha(color, 0.02 * a)),
        ]))
        .border(Border::default().rounded(14.0).width(1.0).color(alpha(color, 0.32 * a)))
}

/// One reading inside a spec panel: sunk, hairline edge.
pub fn stat(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(alpha(Color::BLACK, 0.20))
        .border(Border::default().rounded(12.0).width(1.0).color(alpha(BRONZE_DIM, 0.75)))
}

/// A settings row that reads as a switch: lit gold when it is on.
pub fn switch_row(active: bool, _theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: Some(Background::Color(if active {
            alpha(GOLD, if hovered { 0.10 } else { 0.07 })
        } else if hovered {
            alpha(BG_PANEL_ALT, 0.9)
        } else {
            alpha(Color::BLACK, 0.18)
        })),
        text_color: TEXT_LIGHT,
        border: Border::default().rounded(14.0).width(1.0).color(if active {
            alpha(GOLD, if hovered { 0.65 } else { 0.42 })
        } else if hovered {
            alpha(BRONZE, 0.55)
        } else {
            alpha(BRONZE_DIM, 0.85)
        }),
        ..button::Style::default()
    }
}

/// The track the switch's knob rides on.
pub fn switch_track(active: bool) -> container::Style {
    container::Style::default()
        .background(alpha(if active { GOLD } else { BG_SUNK }, if active { 0.80 } else { 0.95 }))
        .border(Border::default().rounded(999.0).width(1.0).color(alpha(
            if active { GOLD } else { BRONZE_DIM },
            0.9,
        )))
}

/// A stage dot on the install trail.
pub fn stage_dot(color: Color, lit: bool) -> container::Style {
    container::Style::default()
        .background(alpha(color, if lit { 0.95 } else { 0.35 }))
        .border(Border::default().rounded(999.0))
}

/// The confirm action: gold, with the send button's shadow.
pub fn confirm_button(_theme: &Theme, status: button::Status) -> button::Style {
    send_button(_theme, status)
}

// --- library --------------------------------------------------------------
//
// The page the launcher opens on. Covers are drawn from each adventure's hue
// rather than from a picture, so the shelf is complete with no art to ship:
// a tinted plate, one hairline, and the monogram on top.

/// A cover plate: three stops of the same hue, darkening downward.
pub fn cover(hue: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        container::Style::default()
            .background(vertical(&[
                (0.0, hsv(hue, 0.34, 0.38)),
                (0.55, hsv(hue, 0.40, 0.23)),
                (1.0, hsv(hue, 0.44, 0.13)),
            ]))
            .border(Border::default().rounded(12.0).width(1.0).color(alpha(hsv(hue, 0.45, 0.80), 0.32)))
    }
}

/// A row in the catalogue. `sweep` is the eased hover, `0.0` to `1.0`: the
/// tint comes in from the left and the hairline warms with it.
pub fn library_row(sweep: f32, hue: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        let sweep = sweep.clamp(0.0, 1.0);
        let lit = hsv(hue, 0.30, 0.30);

        container::Style::default()
            .background(horizontal(&[
                (0.0, blend(BG_PANEL_ALT, lit, 0.55 * sweep)),
                (0.62, blend(BG_PANEL, lit, 0.18 * sweep)),
                (1.0, alpha(BG_PANEL, 0.94)),
            ]))
            .border(
                Border::default()
                    .rounded(14.0)
                    .width(1.0)
                    .color(alpha(blend(BRONZE_DIM, hsv(hue, 0.45, 0.78), sweep), 0.65 + 0.35 * sweep)),
            )
    }
}

/// A story in progress, in the row under the banner: quieter than a catalogue
/// row, with the hue of its own cover.
pub fn shelf_card(hue: f32, glow: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        container::Style::default()
            .background(vertical(&[
                (0.0, alpha(BG_PANEL, 0.96)),
                (1.0, blend(BG_PANEL, hsv(hue, 0.28, 0.16), 0.35 + 0.25 * glow)),
            ]))
            .border(
                Border::default()
                    .rounded(14.0)
                    .width(1.0)
                    .color(alpha(blend(BRONZE_DIM, hsv(hue, 0.40, 0.70), 0.35 + 0.45 * glow), 0.85)),
            )
    }
}

/// A filter chip. Lit gold when it is the active filter.
pub fn chip(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

        button::Style {
            background: Some(Background::Color(if selected {
                alpha(GOLD, if hovered { 0.20 } else { 0.15 })
            } else if hovered {
                alpha(BG_PANEL_ALT, 0.95)
            } else {
                alpha(BG_PANEL, 0.75)
            })),
            text_color: if selected {
                GOLD
            } else if hovered {
                TEXT_LIGHT
            } else {
                TEXT_MUTED
            },
            border: Border::default().rounded(999.0).width(1.0).color(if selected {
                alpha(GOLD, 0.55)
            } else {
                alpha(BRONZE_DIM, 0.9)
            }),
            ..button::Style::default()
        }
    }
}

/// The order tabs, right of the chips: no border, just weight.
pub fn sort_tab(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

        button::Style {
            background: Some(Background::Color(if hovered && !selected {
                alpha(Color::WHITE, 0.05)
            } else {
                Color::TRANSPARENT
            })),
            text_color: if selected {
                TEXT_LIGHT
            } else if hovered {
                TEXT_MUTED
            } else {
                TEXT_DIM
            },
            border: Border::default().rounded(8.0).width(1.0).color(if selected {
                alpha(GOLD, 0.35)
            } else {
                Color::TRANSPARENT
            }),
            ..button::Style::default()
        }
    }
}

/// The rail's "write your own" card: the one surface on the page allowed to
/// be gold.
pub fn create_card(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(vertical(&[
            (0.0, alpha(GOLD, 0.10)),
            (1.0, alpha(GOLD, 0.03)),
        ]))
        .border(Border::default().rounded(16.0).width(1.0).color(alpha(GOLD, 0.38)))
        .shadow(Shadow {
            color: alpha(GOLD, 0.10),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        })
}

/// A starting-point row in the create sheet: a radio without a widget.
pub fn start_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let mut style = switch_row(selected, theme, if hovered {
            button::Status::Hovered
        } else {
            button::Status::Active
        });
        // The sheet sits on a dim; keep the unselected rows darker than the page.
        if !selected {
            style.background = Some(Background::Color(alpha(Color::BLACK, if hovered { 0.28 } else { 0.20 })));
        }
        style
    }
}

/// A small status pill: "new", "trending", "12 open".
pub fn badge(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        container::Style::default()
            .background(alpha(color, 0.14))
            .border(Border::default().rounded(999.0).width(1.0).color(alpha(color, 0.42)))
    }
}
