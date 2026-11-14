mod cast;
mod composer;
mod header;
mod look;
mod meta;
mod transcript;

use iced::widget::{column, container, row};
use iced::{Element, Length, Task};

use crate::backend::application::adventure_manager::AdventureManager;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::{Act, Fact, State as SceneState};
use crate::backend::domain::message::Message as MessageLine;
use crate::ui::pages::chat::cast::cast_rail;
use crate::ui::pages::chat::header::header;
use crate::ui::pages::chat::meta::meta_rail;
use crate::ui::pages::chat::transcript::transcript;
use crate::ui::pages::{Event, Page};
use crate::ui::theme;

#[derive(Debug, Clone)]
pub enum Message {
    BackRequested,
    InputMessageChanged(String),
    Send,
}

pub struct State {
    pub adventure_manager: AdventureManager,
    pub adventure: Option<Adventure>,
    pub act: Option<Act>,
    pub scene_state: Option<SceneState>,
    pub facts: Vec<Fact>,
    pub messages: Vec<MessageLine>,

    pub input_message: String,
    pub awaiting_reply: bool,
}

impl State {
    pub fn boot(adventure_manager: AdventureManager) -> (Self, Task<Message>) {
        let snapshot = adventure_manager.current_snapshot();

        (
            Self {
                adventure_manager,
                adventure: snapshot.current_adventure,
                act: snapshot.current_act,
                scene_state: snapshot.current_state,
                facts: snapshot.current_facts,
                messages: Vec::new(),
                input_message: String::new(),
                awaiting_reply: false,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::BackRequested => return (Task::none(), Event::Show(Page::MainHub)),
            Message::InputMessageChanged(value) => self.input_message = value,
            Message::Send => {}
        }

        (Task::none(), Event::None)
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            column![
                header(self),
                row![cast_rail(self), transcript(self), meta_rail(self)]
                    .spacing(14.0)
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .spacing(14.0)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .padding([18.0, 22.0])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::page)
        .into()
    }
}
