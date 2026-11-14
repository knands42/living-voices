use crate::ui::pages::main_hub::{Message, State};
use crate::ui::{common, theme};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

const SIDE: f32 = 320.0;

pub fn header(home_state: &State) -> Element<'_, Message> {
    let mark = container(
        text("LV")
            .size(13.0)
            .color(theme::alpha(theme::TEXT_LIGHT, 0.92))
            .font(common::brand()),
    );

    let identity = row![
        mark,
        column![
            text("Living Voices")
                .size(21.0)
                .color(theme::GOLD)
                .font(common::brand()),
            text("stories that answer back")
                .size(10.5)
                .color(theme::TEXT_DIM),
        ]
        .spacing(1.0)
    ]
    .spacing(10.0)
    .align_y(Alignment::Center);

    let search = text_input("Search adventures, people, worlds", &home_state.query)
        .on_input(|value| Message::SearchChanged(value))
        .padding([10.0, 14.0])
        .size(13.0)
        .style(theme::composer);


    let setup = button(text("Setup").size(11.5))
        .padding([7.0, 13.0])
        .style(theme::back_button)
        .on_press(Message::SetupRequested);


    let who = row![setup, common::initials("D", 42.0, 34.0, 0.55)]
        .spacing(10.0)
        .align_y(Alignment::Center);

    row![
        container(identity)
            .width(Length::Fixed(SIDE))
            .align_x(Alignment::Start),
        container(container(search).width(Length::Fixed(380.0)))
            .width(Length::Fill)
            .align_x(Alignment::Center),
        container(who)
            .width(Length::Fixed(SIDE))
            .align_x(Alignment::End),
    ]
    .spacing(16.0)
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .into()
}
