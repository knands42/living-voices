use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

use crate::ui::pages::chat::markup::{Look, has_markup, rich};
use crate::ui::pages::chat::{Message, OngoingAdventure};
use crate::ui::{common, theme};

const PREVIEW_SIZE: f32 = 13.5;

pub fn composer(chat_state: &OngoingAdventure) -> Element<'_, Message> {
    let can_send = !chat_state.input_message.trim().is_empty() && !chat_state.awaiting_reply;

    let input = text_input("Speak, or do something…", chat_state.input_message.as_str())
        .on_input(Message::InputMessageChanged)
        .on_submit(Message::Send)
        .padding([13.0, 15.0])
        .size(14.5)
        .style(theme::composer);

    let send = button(text("Send  ↵").size(14.0).font(common::semibold()))
        .padding([12.0, 20.0])
        .style(theme::send_button)
        .on_press_maybe(can_send.then_some(Message::Send));

    let mut stack = column![].spacing(8.0);

    // Always push a slot: a conditional child shifts the input's position in the
    // widget tree and iced drops its focus state.
    stack = stack.push(if has_markup(chat_state.input_message.as_str()) {
        preview(chat_state.input_message.as_str())
    } else {
        Space::new().into()
    });

    stack
        .push(
            row![input.width(Length::Fill), send]
                .spacing(9.0)
                .align_y(Alignment::Center),
        )
        .into()
}

fn preview(draft: &str) -> Element<'_, Message> {
    container(rich::<Message>(draft, &Look::preview(PREVIEW_SIZE)))
        .padding([7.0, 14.0])
        .width(Length::Fill)
        .style(theme::inset)
        .into()
}
