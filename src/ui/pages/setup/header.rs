use crate::ui::pages::setup::{Message, Phase, State};
use crate::ui::{common, theme};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

const SIDE_LENGTH: f32 = 210.0;

pub fn header(setup_state: &State) -> Element<'_, Message> {
    let back = button(text("← Adventures Hub").size(13.5))
        .padding([9.0, 16.0])
        .style(theme::back_button)
        .on_press(Message::BackRequested);

    let titles = column![
        text("Setup")
            .size(30.0)
            .color(theme::GOLD)
            .font(common::brand()),
        text("Before the story starts, a word about images")
            .size(13.0)
            .color(theme::TEXT_MUTED),
        text("one-time setup").size(11.0).color(theme::TEXT_DIM),
    ]
    .spacing(4.0)
    .align_x(Alignment::Center);

    row![
        container(back)
            .width(Length::Fixed(SIDE_LENGTH))
            .align_x(Alignment::Start),
        container(titles)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        container(status(setup_state))
            .width(Length::Fixed(SIDE_LENGTH))
            .align_x(Alignment::End),
    ]
    .spacing(16.0)
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .into()
}

/// The quiet pill that says what the machine is doing, and which step it is on.
fn status(setup_state: &State) -> Element<'_, Message> {
    let (tint, label, stage) = match &setup_state.phase {
        Phase::Asking => (theme::AMBER, "waiting for you", "step 1 of 2 · install"),
        Phase::Installing => (theme::GOLD, "downloading", "step 1 of 2 · install"),
        Phase::Starting => (theme::GOLD, "starting up", "step 2 of 2 · start"),
        Phase::Running => (theme::SEA, "the engines are running", "ready"),
        Phase::Failed(_) => (theme::ROSE, "stopped", "step 1 of 2 · install"),
    };

    let pill = container(
        row![
            common::dot(7.0, tint),
            text(label).size(11.0).color(theme::TEXT_MUTED)
        ]
        .spacing(8.0)
        .align_y(Alignment::Center),
    )
    .padding([7.0, 12.0])
    .style(theme::inset);

    column![pill, text(stage).size(10.5).color(theme::TEXT_DIM),]
        .spacing(5.0)
        .align_x(Alignment::End)
        .into()
}
