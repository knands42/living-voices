use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::pages::chat::cast::CAST_LENGTH;
use crate::ui::pages::chat::{Message, OngoingAdventure};
use crate::ui::{common, theme};

pub fn header(chat_state: &OngoingAdventure, show_cast: bool) -> Element<'_, Message> {
    let back = button(text("← Adventures Hub").size(11.5))
        .padding([7.0, 13.0])
        .style(theme::back_button)
        .on_press(Message::BackRequested);

    let identity = column![
        text(title(chat_state))
            .size(21.0)
            .color(theme::GOLD)
            .font(common::brand()),
        text(headline(chat_state))
            .size(14.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .wrapping(Wrapping::Word),
    ]
    .spacing(2.0)
    .align_x(Alignment::Center);

    let mut bar = row![].spacing(14.0).align_y(Alignment::Center);

    if show_cast {
        bar = bar.push(
            container(back)
                .width(Length::Fixed(CAST_LENGTH))
                .align_x(Alignment::Start),
        );
    } else {
        bar = bar.push(back);
    }

    bar = bar.push(
        container(identity)
            .width(Length::Fill)
            .align_x(Alignment::Center),
    );

    if show_cast {
        bar = bar.push(Space::new().width(Length::Fixed(CAST_LENGTH)));
    }

    bar.into()
}

fn title(chat_state: &OngoingAdventure) -> &str {
    chat_state.adventure.title.as_str()
}

fn headline(chat_state: &OngoingAdventure) -> &str {
    chat_state.adventure.headline.as_str()
}
