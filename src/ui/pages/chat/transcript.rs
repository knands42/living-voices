use iced::widget::text::Wrapping;
use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::backend::domain::message::Message as MessageLine;
use crate::ui::pages::chat::composer::composer;
use crate::ui::pages::chat::look::{AVATAR, initials_of, name_hue};
use crate::ui::pages::chat::top::top;
use crate::ui::pages::chat::{Message, State};
use crate::ui::{common, theme};

const BUBBLE_LENGTH: f32 = 460.0;
const NARRATOR_LENGTH: f32 = 560.0;

pub fn transcript(chat_state: &State) -> Element<'_, Message> {
    let mut lines = column![].spacing(16.0).width(Length::Fill);

    if chat_state.messages.is_empty() && !chat_state.awaiting_reply {
        lines = lines.push(blank_page());
    }

    for message in &chat_state.messages {
        lines = lines.push(line(chat_state, message));
    }

    if chat_state.awaiting_reply {
        lines = lines.push(typing());
    }

    let history = scrollable(container(lines).padding([4.0, 8.0]))
        .height(Length::Fill)
        .anchor_bottom();

    common::panel(
        column![top(chat_state), history, composer(chat_state)]
            .spacing(12.0)
            .height(Length::Fill),
        16.0,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn blank_page() -> Element<'static, Message> {
    container(
        column![
            text("◆").size(11.0).color(theme::alpha(theme::GOLD, 0.55)),
            text("The page is blank. Say what you do.")
                .size(12.5)
                .color(theme::TEXT_DIM)
                .font(common::italic()),
        ]
        .spacing(6.0)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([48.0, 0.0])
    .align_x(Alignment::Center)
    .into()
}

fn line<'a>(chat_state: &'a State, message: &'a MessageLine) -> Element<'a, Message> {
    if message.narrator_id.is_some() {
        return narrator(message);
    }

    speech(chat_state, message)
}

fn narrator(message: &MessageLine) -> Element<'_, Message> {
    let card = container(
        column![
            text("◆").size(9.0).color(theme::alpha(theme::AMBER, 0.9)),
            text(message.text.as_str())
                .size(12.5)
                .color(theme::TEXT_MUTED)
                .font(common::italic())
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
        ]
        .spacing(5.0)
        .align_x(Alignment::Center),
    )
    .padding([10.0, 16.0])
    .max_width(NARRATOR_LENGTH)
    .style(move |_theme| theme::narrator(1.0));

    container(card)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

fn speech<'a>(chat_state: &'a State, message: &'a MessageLine) -> Element<'a, Message> {
    let hue = name_hue(message.speaker_name.as_str());
    let is_player = chat_state
        .adventure
        .as_ref()
        .and_then(|adventure| adventure.characters.iter().find(|cast| cast.is_player))
        .is_some_and(|player| message.character_id == Some(player.id));

    let mut heading = row![
        text(message.speaker_name.as_str())
            .size(11.5)
            .color(theme::hsv(hue, 0.45, 0.95))
            .font(common::semibold()),
        text(message.created_at.format("%H:%M").to_string())
            .size(9.5)
            .color(theme::TEXT_DIM),
    ]
    .spacing(7.0)
    .align_y(Alignment::Center);

    if message.ooc {
        heading = heading.push(
            text("ooc")
                .size(9.0)
                .color(theme::AMBER)
                .font(common::semibold()),
        );
    }

    let words = text(message.text.as_str())
        .size(13.0)
        .color(theme::TEXT_LIGHT)
        .width(Length::Fill)
        .wrapping(Wrapping::Word);

    let bubble = container(words)
        .padding([9.0, 13.0])
        .max_width(BUBBLE_LENGTH)
        .style(move |_theme| theme::bubble(hue, is_player, 1.0));

    let initials = initials_of(message.speaker_name.as_str());
    let avatar = common::initials(&initials, hue, AVATAR, 0.55);

    if is_player {
        row![
            Space::new().width(Length::Fill),
            column![
                container(heading)
                    .width(Length::Fill)
                    .align_x(Alignment::End),
                bubble,
            ]
            .spacing(4.0)
            .align_x(Alignment::End),
            avatar,
        ]
        .spacing(9.0)
        .align_y(Alignment::Start)
        .into()
    } else {
        row![
            avatar,
            column![heading, bubble].spacing(4.0).width(Length::Fill),
        ]
        .spacing(9.0)
        .align_y(Alignment::Start)
        .into()
    }
}

fn typing() -> Element<'static, Message> {
    let mut dots = row![].spacing(4.0).align_y(Alignment::Center);

    for fade in [0.85, 0.55, 0.30] {
        dots = dots.push(common::dot(5.0, theme::alpha(theme::GOLD, fade)));
    }

    row![
        dots,
        text("writing…")
            .size(10.5)
            .color(theme::TEXT_DIM)
            .font(common::italic()),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center)
    .into()
}
