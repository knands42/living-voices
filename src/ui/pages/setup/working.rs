use crate::ui::pages::setup::{Message, Progress};
use crate::ui::{common, theme};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn working<'a>(
    title: &'a str,
    stage: usize,
    progress: Option<&Progress>,
    spinner: f32,
) -> Element<'a, Message> {
    let percent = progress
        .and_then(Progress::percent)
        .map(|value| format!("{value}%"))
        .unwrap_or_default();

    let mut body = column![
        row![
            text(title)
                .size(15.5)
                .color(theme::GOLD)
                .font(common::semibold()),
            Space::new().width(Length::Fill),
            text(percent).size(11.5).color(theme::BRONZE),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(10.0)
    .width(Length::Fill);

    if let Some(progress) = progress {
        body = body.push(common::meter(
            progress.ratio(),
            theme::GOLD,
            theme::alpha(theme::BG_SUNK, 0.95),
            6.0,
        ));
        body = body.push(
            text(progress.label())
                .size(11.5)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(text::Wrapping::Word),
        );
    }

    body = body.push(common::divider());
    body = body.push(trail(stage, spinner));

    container(body)
        .padding(18.0)
        .width(Length::Fill)
        .style(theme::panel)
        .into()
}

/// The done stages keep a gold dot, the current one spins.
fn trail(current: usize, spinner: f32) -> Element<'static, Message> {
    let stages = ["Installing", "Ready"];

    let mut line = row![].spacing(18.0).align_y(Alignment::Center);

    for (index, label) in stages.iter().enumerate() {
        let (marker, text_color) = if index < current {
            (common::dot(6.0, theme::GOLD), theme::TEXT_LIGHT)
        } else if index == current {
            (spinner_widget(spinner), theme::TEXT_LIGHT)
        } else {
            (
                common::dot(6.0, theme::alpha(theme::TEXT_DIM, 0.5)),
                theme::TEXT_DIM,
            )
        };

        line = line.push(
            row![marker, text(*label).size(11.0).color(text_color)]
                .spacing(6.0)
                .align_y(Alignment::Center),
        );
    }

    line.into()
}

fn spinner_widget(angle: f32) -> Element<'static, Message> {
    common::spinner(
        angle,
        10.0,
        theme::AMBER,
        theme::alpha(theme::BRONZE_DIM, 0.5),
    )
}
