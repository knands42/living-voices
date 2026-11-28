use crate::ui::pages::setup::{Message, Progress};
use crate::ui::{common, theme};
use iced::widget::canvas;
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Color, Element, Length, Point};

pub fn working<'a>(
    title: &'a str,
    stage: usize,
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
    body = body.push(trail(stage, spinner));

    container(body)
        .padding(18.0)
        .width(Length::Fill)
        .style(theme::panel)
        .into()
}

/// The done stages keep a gold dot, the current one spins.
fn trail(current: usize, spinner: f32) -> Element<'static, Message> {
    let stages = ["Installing", "Ready"];

    let mut line = row![].spacing(18.0).align_y(Alignment::Center);

    for (index, label) in stages.iter().enumerate() {
        let (marker, text_color) = if index < current {
            (common::dot(6.0, theme::GOLD), theme::TEXT_LIGHT)
        } else if index == current {
            (spinner_widget(spinner), theme::TEXT_LIGHT)
        } else {
            (
                common::dot(6.0, theme::alpha(theme::TEXT_DIM, 0.5)),
                theme::TEXT_DIM,
            )
        };

        line = line.push(
            row![marker, text(*label).size(11.0).color(text_color)]
                .spacing(6.0)
                .align_y(Alignment::Center),
        );
    }

    line.into()
}

fn spinner_widget(angle: f32) -> Element<'static, Message> {
    canvas(Spinner {
        angle,
        color: theme::AMBER,
        track: theme::alpha(theme::BRONZE_DIM, 0.5),
    })
    .width(Length::Fixed(10.0))
    .height(Length::Fixed(10.0))
    .into()
}

struct Spinner {
    angle: f32,
    color: Color,
    track: Color,
}

impl<Message> canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = frame.center();
        let radius = bounds.size().width.min(bounds.size().height) / 2.0 - 1.5;

        frame.stroke(
            &canvas::Path::circle(center, radius),
            canvas::Stroke::default()
                .with_color(self.track)
                .with_width(1.5),
        );

        let dot = Point::new(
            center.x + radius * self.angle.cos(),
            center.y + radius * self.angle.sin(),
        );
        frame.fill(&canvas::Path::circle(dot, 1.2), self.color);

        vec![frame.into_geometry()]
    }
}
