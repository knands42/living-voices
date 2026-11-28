use crate::ui::pages::setup::{Message, Phase, State};
use crate::ui::{common, theme};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn question(setup_state: &State) -> Element<'_, Message> {
    let active = setup_state.media_engine;
    let failed = matches!(&setup_state.phase, Phase::Failed(_));

    // A settings row rather than a bare toggler: the whole row is the target,
    // and the knob is drawn from containers so it carries the palette.
    let switch = button(
        row![
            column![
                text("Media engine (ComfyUI)")
                    .size(13.5)
                    .color(theme::TEXT_LIGHT)
                    .font(common::semibold()),
                text("ComfyUI renders the pictures the narrator describes")
                    .size(11.0)
                    .color(theme::TEXT_DIM),
            ]
            .spacing(3.0)
            .width(Length::Fill),
            knob(active),
        ]
        .spacing(14.0)
        .align_y(Alignment::Center),
    )
    .padding([11.0, 13.0])
    .width(Length::Fill)
    .style(move |t, status| theme::switch_row(active, t, status))
    .on_press(Message::MediaToggled(!active));

    let action = button(text(action_label(setup_state)).size(13.5))
        .padding([10.0, 20.0])
        .style(theme::confirm_button)
        .on_press(Message::Confirmed);

    let top = row![
        common::subheading("The media engine"),
        Space::new().width(Length::Fill),
        text("you can change this later")
            .size(10.5)
            .color(theme::TEXT_DIM),
    ]
    .align_y(Alignment::Center);

    let footer = text(footer_text(setup_state))
        .size(11.5)
        .color(if failed { theme::ROSE } else { theme::TEXT_DIM })
        .width(Length::Fill)
        .wrapping(text::Wrapping::Word);

    container(
        column![
            top,
            switch,
            row![footer, action]
                .spacing(16.0)
                .align_y(Alignment::Center),
        ]
        .spacing(12.0)
        .width(Length::Fill),
    )
    .padding(18.0)
    .width(Length::Fill)
    .style(theme::panel)
    .into()
}

fn action_label(setup_state: &State) -> &'static str {
    if matches!(&setup_state.phase, Phase::Failed(_)) {
        return "Try again";
    }
    if setup_state.media_engine {
        "Use images"
    } else {
        "Go without images"
    }
}

fn footer_text(setup_state: &State) -> String {
    match (&setup_state.phase, setup_state.notice) {
        (_, Some(notice)) => notice.to_string(),
        (Phase::Failed(err), None) => format!("The engines could not start: {err}"),
        _ => "You can come back to this page any time.".to_string(),
    }
}

/// The switch's knob on its track.
fn knob(active: bool) -> Element<'static, Message> {
    let bead = container(
        Space::new()
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(12.0)),
    )
    .style(move |_theme| {
        theme::stage_dot(
            if active {
                theme::BG_DEEPEST
            } else {
                theme::TEXT_DIM
            },
            true,
        )
    });

    container(
        container(bead)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(4.0)
            .align_x(if active {
                Alignment::End
            } else {
                Alignment::Start
            })
            .align_y(Alignment::Center),
    )
    .width(Length::Fixed(40.0))
    .height(Length::Fixed(20.0))
    .style(move |_theme| theme::switch_track(active))
    .into()
}
