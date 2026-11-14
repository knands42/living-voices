use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::backend::domain::memory::{Act, ActStatus};
use crate::ui::pages::chat::cast::CAST_LENGTH;
use crate::ui::pages::chat::{Message, State};
use crate::ui::{common, theme};

const ACT_LENGTH: f32 = 220.0;

pub fn header(chat_state: &State) -> Element<'_, Message> {
    let back = button(text("← Back").size(11.5))
        .padding([7.0, 13.0])
        .style(theme::back_button)
        .on_press(Message::BackRequested);

    let identity = column![
        text(title(chat_state))
            .size(21.0)
            .color(theme::GOLD)
            .font(common::brand()),
        text(headline(chat_state))
            .size(11.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
    ]
    .spacing(2.0)
    .align_x(Alignment::Center);

    row![
        container(back)
            .width(Length::Fixed(CAST_LENGTH))
            .align_x(Alignment::Start),
        container(identity)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        container(act_chip(chat_state.act.as_ref()))
            .width(Length::Fixed(CAST_LENGTH))
            .align_x(Alignment::End),
    ]
    .spacing(14.0)
    .align_y(Alignment::Center)
    .into()
}

fn title(chat_state: &State) -> &str {
    chat_state
        .adventure
        .as_ref()
        .map_or("No adventure open", |adventure| adventure.title.as_str())
}

fn headline(chat_state: &State) -> &str {
    chat_state
        .adventure
        .as_ref()
        .map_or("Pick an adventure from the shelf", |adventure| {
            adventure.headline.as_str()
        })
}

fn act_chip(act: Option<&Act>) -> Element<'_, Message> {
    let Some(act) = act else {
        return Space::new().into();
    };

    let status = status_color(act.status);
    let label = format!("Act {}", act.ordinal);

    container(
        column![
            row![common::dot(5.0, status), common::eyebrow(&label),]
                .spacing(6.0)
                .align_y(Alignment::Center),
            text(act.goal.as_str())
                .size(11.0)
                .color(theme::TEXT_LIGHT)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            text(act.status.as_str().to_uppercase())
                .size(9.0)
                .color(status)
                .font(common::semibold()),
        ]
        .spacing(4.0),
    )
    .padding([9.0, 12.0])
    .max_width(ACT_LENGTH)
    .style(theme::stat)
    .into()
}

fn status_color(status: ActStatus) -> iced::Color {
    match status {
        ActStatus::Active => theme::GOLD,
        ActStatus::Resolved => theme::SEA,
        ActStatus::Failed => theme::ROSE,
        ActStatus::Unstarted | ActStatus::Bypassed => theme::TEXT_DIM,
    }
}
