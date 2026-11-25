use iced::widget::text::Wrapping;
use iced::widget::{container, row, text};
use iced::{Alignment, Color, Element};

use crate::backend::domain::memory::{Act, ActStatus};
use crate::ui::pages::chat::{Message, OngoingAdventure};
use crate::ui::{common, theme};

pub fn top(chat_state: &OngoingAdventure) -> Element<'_, Message> {
    row![act_chip(&chat_state.act),]
        .spacing(12.0)
        .align_y(Alignment::Center)
        .into()
}

fn act_chip(act: &Act) -> Element<'_, Message> {
    container(
        row![
            text(format!("ACT {}", roman(usize::from(act.ordinal) + 1)))
                .size(12.0)
                .color(status_color(act.status))
                .font(common::semibold()),
            text(act.goal.as_str())
                .size(12.5)
                .color(theme::TEXT_DIM)
                .wrapping(Wrapping::Word),
        ]
        .spacing(8.0)
        .align_y(Alignment::Center),
    )
    .padding([5.0, 10.0])
    .max_width(ACT_LENGTH)
    .style(theme::inset)
    .into()
}

const ACT_LENGTH: f32 = 320.0;

fn status_color(status: ActStatus) -> Color {
    match status {
        ActStatus::Active => theme::GOLD,
        ActStatus::Resolved => theme::SEA,
        ActStatus::Failed => theme::ROSE,
        ActStatus::Unstarted | ActStatus::Bypassed => theme::TEXT_DIM,
    }
}

fn roman(mut number: usize) -> String {
    const NUMERALS: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];

    let mut out = String::new();
    for (value, symbol) in NUMERALS {
        while number >= value {
            out.push_str(symbol);
            number -= value;
        }
    }
    out
}
