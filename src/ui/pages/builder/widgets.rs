use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, text, text_editor, text_input, tooltip};
use iced::{Alignment, Background, Border, Element, Length, alignment};

use crate::ui::{common, theme};

/// The heading at the top of a section, with its muted subtitle.
pub fn section_header<'a, M: Clone + 'a>(title: &'a str, subtitle: &'a str) -> Element<'a, M> {
    column![
        text(title)
            .size(22.0)
            .color(theme::GOLD)
            .font(common::brand()),
        text(subtitle)
            .size(12.5)
            .color(theme::TEXT_MUTED)
            .wrapping(Wrapping::Word),
    ]
    .spacing(3.0)
    .width(Length::Fill)
    .into()
}

/// A gold pill. `enabled` greys it and takes the press away.
pub fn pill<'a, M: Clone + 'a>(
    label: &'a str,
    width: f32,
    enabled: bool,
    message: M,
) -> Element<'a, M> {
    button(
        container(
            text(label)
                .size(12.5)
                .font(common::semibold())
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Center),
        )
        .padding([8.0, 0.0])
        .width(Length::Fixed(width)),
    )
    .style(theme::send_button)
    .on_press_maybe(enabled.then_some(message))
    .into()
}

/// A small caps caption above a field.
pub fn caption<'a, M: Clone + 'a>(label: &str) -> Element<'a, M> {
    text(label.to_uppercase())
        .size(11.0)
        .color(theme::TEXT_MUTED)
        .font(common::semibold())
        .into()
}

/// A single-line field, styled like the composer so the forms read as one product.
pub fn field<'a, M: Clone + 'a>(
    value: &'a str,
    placeholder: &'a str,
    on_input: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    text_input(placeholder, value)
        .padding([10.0, 14.0])
        .size(14.5)
        .style(theme::composer)
        .on_input(on_input)
        .into()
}

/// A caption over its field, sized to share a row with its neighbours.
pub fn labelled<'a, M: Clone + 'a>(label: &'a str, field: Element<'a, M>) -> Element<'a, M> {
    column![caption(label), field]
        .spacing(8.0)
        .width(Length::FillPortion(1))
        .into()
}

/// A labelled field whose caption carries an info icon explaining what the field feeds.
pub fn hinted<'a, M: Clone + 'a>(
    label: &'a str,
    hint: &'a str,
    field: Element<'a, M>,
) -> Element<'a, M> {
    column![
        row![caption(label), info(hint)]
            .spacing(6.0)
            .align_y(Alignment::Center),
        field,
    ]
    .spacing(8.0)
    .width(Length::FillPortion(1))
    .into()
}

fn info<'a, M: Clone + 'a>(hint: &'a str) -> Element<'a, M> {
    let badge = container(
        text("i")
            .size(9.0)
            .color(theme::BG_PANEL)
            .font(common::semibold()),
    )
    .width(Length::Fixed(14.0))
    .height(Length::Fixed(14.0))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(|_theme| container::Style {
        background: Some(Background::Color(theme::TEXT_MUTED)),
        border: Border::default().rounded(7.0),
        ..container::Style::default()
    });

    let body = container(
        text(hint)
            .size(11.5)
            .color(theme::TEXT_LIGHT)
            .width(Length::Fixed(230.0)),
    )
    .padding(10.0)
    .style(|_theme| container::Style {
        background: Some(Background::Color(theme::BG_PANEL)),
        border: Border::default()
            .rounded(8.0)
            .width(1.0)
            .color(theme::BRONZE_DIM),
        ..container::Style::default()
    });

    tooltip(badge, body, tooltip::Position::Top).gap(6.0).into()
}

/// A multi-line field.
pub fn editor<'a, M: Clone + 'a>(
    content: &'a text_editor::Content,
    min_height: f32,
    on_action: impl Fn(text_editor::Action) -> M + 'a,
) -> Element<'a, M> {
    text_editor(content)
        .padding(12.0)
        .size(14.5)
        .min_height(min_height)
        .wrapping(Wrapping::Word)
        .style(|_theme, _status| editor_style())
        .on_action(on_action)
        .into()
}

fn editor_style() -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(theme::alpha(theme::BG_SUNK, 0.92)),
        border: Border::default()
            .rounded(12.0)
            .width(1.0)
            .color(theme::BRONZE_DIM),
        placeholder: theme::TEXT_DIM,
        value: theme::TEXT_LIGHT,
        selection: theme::alpha(theme::BRONZE, 0.85),
    }
}

/// The bordered panel a section sets its fields into.
pub fn surface<'a, M: Clone + 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .padding(28.0)
        .width(Length::Fill)
        .style(|_theme| {
            container::Style::default()
                .background(theme::BG_PANEL)
                .border(
                    Border::default()
                        .rounded(16.0)
                        .width(1.0)
                        .color(theme::BRONZE_DIM),
                )
        })
        .into()
}

/// The quiet panel a section shows while its list is empty.
pub fn empty_state<'a, M: Clone + 'a>(label: &str) -> Element<'a, M> {
    container(text(label.to_string()).size(12.5).color(theme::TEXT_MUTED))
        .padding(28.0)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(alignment::Vertical::Center)
        .style(theme::panel)
        .into()
}

/// A panel the sections set their cards and act rows into.
pub fn card<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    width: Length,
    edge: iced::Color,
) -> Element<'a, M> {
    container(content)
        .padding(14.0)
        .width(width)
        .style(move |_theme| {
            container::Style::default()
                .background(theme::BG_PANEL)
                .border(Border::default().rounded(14.0).width(1.0).color(edge))
        })
        .into()
}

/// A rounded tag, used for genres, traits and act statuses.
pub fn tag<'a, M: Clone + 'a>(label: &'a str, tint: iced::Color) -> Element<'a, M> {
    container(
        text(label)
            .size(10.5)
            .color(tint)
            .align_x(alignment::Horizontal::Center),
    )
    .padding([3.0, 8.0])
    .style(theme::badge(tint))
    .into()
}
