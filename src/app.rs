use sqlx::SqlitePool;

use crate::ai::integrations::text::chat_handler::ChatHandler;
use crate::ai::integrations::text::narrator_handler::NarratorHandler;
use crate::ai::{ActClose, MemoryHandler};
use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
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
    C: ChatPort + Clone,
    N: NarratorPort + Clone,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
    P: PromptPort + Clone,
> {
    pub adventures: AdventureManager,
    pub characters: CharacterManager<P>,
    pub messages: MessageManager<C, N, M, A>,
}

pub type DefaultContext =
    AppContext<ChatHandler, NarratorHandler, MemoryHandler, ActClose, ChatHandler>;

pub struct App {
    ctx: DefaultContext,
}

impl App {
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
        let distiller = MemoryHandler::new(memory_model.clone());
        let closer = ActClose::new(memory_model);
        let narrator_handler = NarratorHandler::new(chat_model.clone());
        let chat_handler = ChatHandler::new(chat_model);

        // application
        let director = Director::new(director_settings);

        let adventure_manager = AdventureManager::new(state.clone(), pool.clone());
        let character_manager =
            CharacterManager::new(state.clone(), pool.clone(), chat_handler.clone());
        let memory_manager = MemoryManager::new(
            state.clone(),
            pool.clone(),
            distiller,
            closer,
            memory_settings,
        );
        let message_manager = MessageManager::new(
            state.clone(),
            pool.clone(),
            chat_handler,
            narrator_handler,
            director,
            memory_manager,
        );

        Self {
            ctx: AppContext {
                adventures: adventure_manager,
                characters: character_manager,
                messages: message_manager,
            },
        }
    }

    pub fn ctx(&self) -> &DefaultContext {
        &self.ctx
    }

    #[cfg(debug_assertions)]
    pub async fn load_adventure(
        &self,
        adventure: crate::backend::domain::adventure::Adventure,
    ) -> Result<(), crate::backend::application::adventure_manager::AdventureManagerError> {
        self.ctx.adventures.start_adventure(adventure.id).await?;

        Ok(())
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        ui::run(self.ctx.clone())
    }
}
