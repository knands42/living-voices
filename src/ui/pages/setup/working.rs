use crate::ui::pages::setup::{Message, Progress};
use crate::ui::{common, theme};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn working<'a>(
    title: &'a str,
    stage: usize,
    progress: Option<&Progress>,
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
    body = body.push(trail(stage));

    container(body)
        .padding(18.0)
        .width(Length::Fill)
        .style(theme::panel)
        .into()
}

/// Install, start, run: three dots that light in turn.
fn trail(current: usize) -> Element<'static, Message> {
    let stages = ["Installing", "Starting", "Running"];

    let mut line = row![].spacing(18.0).align_y(Alignment::Center);

    for (index, label) in stages.iter().enumerate() {
        let (tint, lit) = if index < current {
            (theme::GOLD, true)
        } else if index == current {
            (theme::AMBER, true)
        } else {
            (theme::TEXT_DIM, false)
        };

        line = line.push(
            row![
                common::dot(6.0, theme::alpha(tint, if lit { 0.95 } else { 0.5 })),
                text(*label).size(11.0).color(if lit {
                    theme::TEXT_LIGHT
                } else {
                    theme::TEXT_DIM
                }),
            ]
            .spacing(6.0)
            .align_y(Alignment::Center),
        );
    }

    line.into()
}
