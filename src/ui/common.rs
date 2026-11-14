use crate::ui::{assets, theme};
use iced::widget::{Container, Space, container, image, progress_bar, text};
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

/// A panel title, e.g. "Cast".
pub fn heading<Message: 'static>(label: &str) -> Element<'_, Message> {
    text(label)
        .size(15.5)
        .color(theme::GOLD)
        .font(semibold())
        .into()
}

/// A section title inside a panel, e.g. "Conditions".
pub fn subheading<Message: 'static>(label: &str) -> Element<Message> {
    text(label)
        .size(12.0)
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
pub fn eyebrow<Message: 'static>(label: &str) -> Element<Message> {
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
) -> Element<'_, Message> {
    container(
        text(label)
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
