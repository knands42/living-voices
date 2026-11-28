use crate::ui::pages::setup::{Message, Progress, Stage};
use crate::ui::{common, theme};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn working<'a>(
    title: &'a str,
    stage: Stage,
    steps: &[Stage],
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
    body = body.push(trail(steps, stage, spinner));

    container(body)
        .padding(18.0)
        .width(Length::Fill)
        .style(theme::panel)
        .into()
}

/// Steps already reached keep a gold dot; the one being worked spins.
fn trail(steps: &[Stage], stage: Stage, spinner: f32) -> Element<'static, Message> {
    let mut line = row![].spacing(18.0).align_y(Alignment::Center);

    for &step in steps {
        let (marker, label, text_color) = if step == stage && step != Stage::Ready {
            (
                spinner_widget(spinner),
                step.label_when_progressing(),
                theme::TEXT_LIGHT,
            )
        } else if step <= stage {
            (
                common::dot(6.0, theme::GOLD),
                step.label_when_done(),
                theme::TEXT_LIGHT,
            )
        } else {
            (
                common::dot(6.0, theme::alpha(theme::TEXT_DIM, 0.5)),
                step.label_when_progressing(),
                theme::TEXT_DIM,
            )
        };

        line = line.push(
            row![marker, text(label).size(14.0).color(text_color)]
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
