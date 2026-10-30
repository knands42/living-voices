use ratatui::layout::Rect;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::message::Message;

pub enum Page {
    Main,
    AdventureList,
    Chat { input: String, cursor: usize },
}

pub struct CliState {
    pub page: Page,
    pub adventures: Vec<Adventure>,
    pub status: Option<String>,
    pub messages: Vec<Message>,
    pub hovered_character: Option<usize>,
    pub cast_row_hitboxes: Vec<Rect>,
    pub reply_rx: UnboundedReceiver<ChatEvent>,
    pub reply_tx: UnboundedSender<ChatEvent>,
}

pub enum ChatEvent {
    Reply(Message),
    Failed(String),
}

impl Default for CliState {
    fn default() -> Self {
        let (reply_tx, reply_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            page: Page::Main,
            adventures: vec![],
            status: None,
            messages: Vec::new(),
            hovered_character: None,
            cast_row_hitboxes: Vec::new(),
            reply_rx,
            reply_tx,
        }
    }
}
