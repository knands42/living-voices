use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::ui::{assets, theme};
use iced::widget::{Container, Space, canvas, container, image, progress_bar, text};
use iced::{Alignment, ContentFit, Element, Font, Length};

// Typography
pub fn brand() -> Font {
    Font {
        family: iced::font::Family::Serif,
        weight: iced::font::Weight::Bold,
        ..Font::DEFAULT
    }
}

pub fn semibold() -> Font {
    Font {
        weight: iced::font::Weight::Semibold,
        ..Font::DEFAULT
    }
}

pub fn italic() -> Font {
    Font {
        style: iced::font::Style::Italic,
        ..Font::DEFAULT
    }
}

pub fn initials_of(name: &str) -> String {
    name.chars().take(2).collect::<String>().to_uppercase()
}

pub fn name_hue(name: &str) -> f32 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    (hasher.finish() % 360) as f32
}

/// A panel title, e.g. "Cast".
pub fn heading<Message: 'static>(label: &str) -> Element<'_, Message> {
    text(label)
        .size(17.0)
        .color(theme::GOLD)
        .font(semibold())
        .into()
}

/// A section title inside a panel, e.g. "Conditions".
pub fn subheading<Message: 'static>(label: &str) -> Element<'_, Message> {
    text(label)
        .size(13.0)
        .color(theme::AMBER)
        .font(semibold())
        .into()
}

/// A bordered, shadowed surface.
pub fn panel<'a, Message: 'static>(
    content: impl Into<Element<'a, Message>>,
    padding: f32,
) -> Container<'a, Message> {
    container(content).padding(padding).style(theme::panel)
}

/// A tiny all-caps label, the kind that sits above a number.
pub fn eyebrow<Message: 'static>(label: &str) -> Element<'static, Message> {
    text(label.to_uppercase())
        .size(9.5)
        .color(theme::TEXT_DIM)
        .font(semibold())
        .into()
}

/// Stands in for `PortraitFrame` when there is no picture: a hue-tinted circle
pub fn initials<Message: 'static>(
    label: &str,
    hue: f32,
    size: f32,
    glow: f32,
) -> Element<'static, Message> {
    container(
        text(label.to_string())
            .size(size * 0.34)
            .color(theme::hsv(hue, 0.30, 0.96))
            .font(semibold()),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(move |_theme| theme::portrait(hue, glow))
    .into()
}

/// A round portrait from the generated art, ringed in the character's hue.
pub fn avatar<'a, Message: 'static>(
    file_path: &str,
    hue: f32,
    size: f32,
    glow: f32,
) -> Element<'a, Message> {
    let picture = image(assets::asset_handle(file_path))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .content_fit(ContentFit::Cover)
        .border_radius(size / 2.0);

    container(picture)
        .padding(1.5)
        .style(move |_theme| theme::portrait(hue, glow))
        .into()
}

/// A rounded picture with a hairline frame.
pub fn picture<'a, Message: 'static>(
    file_path: &str,
    width: Length,
    height: f32,
    radius: f32,
    opacity: f32,
) -> Element<'a, Message> {
    image(assets::asset_handle(file_path))
        .width(width)
        .height(Length::Fixed(height))
        .content_fit(ContentFit::Cover)
        .border_radius(radius)
        .opacity(opacity)
        .into()
}

/// A one-pixel divider that fades out at the ends.
pub fn divider<'a, Message: 'static>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(|_theme| {
            container::Style::default().background(theme::horizontal(&[
                (0.0, theme::alpha(theme::BRONZE_DIM, 0.0)),
                (0.5, theme::alpha(theme::BRONZE_DIM, 0.9)),
                (1.0, theme::alpha(theme::BRONZE_DIM, 0.0)),
            ]))
        })
        .into()
}

/// A thin meter
pub fn meter<'a, Message: 'static>(
    value: f32,
    fill: iced::Color,
    track: iced::Color,
    girth: f32,
) -> Element<'a, Message> {
    progress_bar(0.0..=1.0, value.clamp(0.0, 1.0))
        .girth(Length::Fixed(girth))
        .length(Length::Fill)
        .style(theme::bar(fill, track))
        .into()
}

/// A small square swatch, used for severity dots and the live indicator.
pub fn dot<'a, Message: 'static>(size: f32, color: iced::Color) -> Element<'a, Message> {
    container(
        Space::new()
            .width(Length::Fixed(size))
            .height(Length::Fixed(size)),
    )
    .style(move |_theme| {
        container::Style::default()
            .background(color)
            .border(iced::Border::default().rounded(999.0))
    })
    .into()
}

/// A ring with one bead riding it, `angle` radians around.
pub fn spinner<'a, Message: 'static>(
    angle: f32,
    size: f32,
    color: iced::Color,
    track: iced::Color,
) -> Element<'a, Message> {
    canvas(Spinner {
        angle,
        color,
        track,
    })
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .into()
}

struct Spinner {
    angle: f32,
    color: iced::Color,
    track: iced::Color,
}

impl<Message> canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = frame.center();
        let radius = bounds.size().width.min(bounds.size().height) / 2.0 - 1.5;

        frame.stroke(
            &canvas::Path::circle(center, radius),
            canvas::Stroke::default()
                .with_color(self.track)
                .with_width(1.5),
        );

        let bead = iced::Point::new(
            center.x + radius * self.angle.cos(),
            center.y + radius * self.angle.sin(),
        );
        frame.fill(&canvas::Path::circle(bead, 1.2), self.color);

        vec![frame.into_geometry()]
    }
}
