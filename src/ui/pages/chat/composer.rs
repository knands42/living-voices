use iced::widget::{button, row, text, text_input};
use iced::{Alignment, Element, Length};

use crate::ui::pages::chat::{Message, State};
use crate::ui::{common, theme};

pub fn composer(chat_state: &State) -> Element<'_, Message> {
    let can_send = !chat_state.input_message.trim().is_empty() && !chat_state.awaiting_reply;

    let input = text_input("Speak, or do something…", chat_state.input_message.as_str())
        .on_input(Message::InputMessageChanged)
        .on_submit(Message::Send)
        .padding([13.0, 15.0])
        .size(13.5)
        .style(theme::composer);

    let send = button(text("Send  ↵").size(13.0).font(common::semibold()))
        .padding([12.0, 20.0])
        .style(theme::send_button)
        .on_press_maybe(can_send.then_some(Message::Send));

    row![input.width(Length::Fill), send]
        .spacing(9.0)
        .align_y(Alignment::Center)
        .into()
}
