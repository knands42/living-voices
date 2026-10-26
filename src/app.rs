use sqlx::SqlitePool;

use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::ai::{ActClose, MemoryUpdate};
use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::{ChatHandler, NarratorHandler};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::{
    backend::application::{
        adventure_manager::AdventureManager, character_manager::CharacterManager,
        memory_manager::MemoryManager, message_manager::MessageManager, state::SharedState,
    },
    config::{DirectorSettings, MemorySettings, TextModel},
    ui,
};

#[derive(Clone)]
pub struct AppContext<
    C: ChatHandler + Clone,
    N: NarratorHandler + Clone,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
> {
    pub adventures: AdventureManager,
    pub characters: CharacterManager,
    pub messages: MessageManager<C, N, M, A>,
}

pub struct App<
    C: ChatHandler + Clone,
    N: NarratorHandler + Clone,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
> {
    ctx: AppContext<C, N, M, A>,
}

impl App<Chat, Narrator, MemoryUpdate, ActClose> {
    pub fn new(
        pool: SqlitePool,
        memory_model: TextModel,
        chat_model: TextModel,
        memory_settings: MemorySettings,
        director_settings: DirectorSettings,
    ) -> Self {
        // state
        let state = SharedState::new();

        // ai modules — the narrator shares the chat text endpoint/config.
        let distiller = MemoryUpdate::new(memory_model.clone());
        let closer = ActClose::new(memory_model);
        let narrator_handler = Narrator::new(chat_model.clone());
        let chat_handler = Chat::new(chat_model);

        // application
        let director = Director::new(director_settings);

        let adventure_manager = AdventureManager::new(state.clone(), pool.clone());
        let character_manager = CharacterManager::new(state.clone(), pool.clone());
        let memory_manager = MemoryManager::new(state.clone(), pool.clone(), distiller, closer);
        let message_manager = MessageManager::new(
            state.clone(),
            pool.clone(),
            chat_handler,
            narrator_handler,
            director,
            memory_manager,
            memory_settings,
        );

        Self {
            ctx: AppContext {
                adventures: adventure_manager,
                characters: character_manager,
                messages: message_manager,
            },
        }
    }

    pub fn ctx(&self) -> &AppContext<Chat, Narrator, MemoryUpdate, ActClose> {
        &self.ctx
    }
}

impl<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
> App<C, N, M, A>
{
    #[cfg(debug_assertions)]
    pub fn load_adventure(
        &self,
        adventure: crate::backend::domain::adventure::Adventure,
    ) -> Result<(), crate::backend::application::adventure_manager::AdventureManagerError> {
        self.ctx.adventures.set_current_adventure(adventure)
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
