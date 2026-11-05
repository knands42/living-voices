use crate::ai::integrations::text::chat::Chat;
use crate::app::AppContext;
use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::application::character_manager::{CharacterManager, CharacterManagerError};
use crate::backend::application::port::chat_port::{ChatHandler, NarratorHandler};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::domain::character::{Character, CharacterStatus};
use crate::ui::callbacks::adventure_creation::{
    BuilderData, load_builder_data, parse_id, show_builder,
};
use crate::ui::{BuilderGlobals, CharacterGlobals, RootWindow, viewmodels};
use slint::ComponentHandle;
use std::future::Future;
use thiserror::Error;
use tokio::runtime::Handle;
use uuid::Uuid;

const NEW_CHARACTER_NAME: &str = "New character";

#[derive(Error, Debug)]
enum CharacterPageError {
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Adventure(#[from] AdventureManagerError),
    #[error(transparent)]
    Character(#[from] CharacterManagerError),
}

/// Everything a character-page callback needs, cheap to clone into each closure.
#[derive(Clone)]
struct PageEnv {
    handle: Handle,
    adventures: AdventureManager,
    characters: CharacterManager<Chat>,
    ui_weak: slint::Weak<RootWindow>,
}

impl PageEnv {
    /// Runs `work`, reloads the adventure, then refreshes the builder and the character page.
    fn run(
        &self,
        action: &str,
        adventure_id: Uuid,
        character_id: Uuid,
        open_page: bool,
        work: impl Future<Output = Result<(), CharacterPageError>>,
    ) {
        let result = tokio::task::block_in_place(|| {
            self.handle.block_on(async {
                work.await?;
                Ok::<_, CharacterPageError>(
                    load_builder_data(&self.adventures, adventure_id).await?,
                )
            })
        });

        match result {
            Ok(data) => self.show(&data, character_id, open_page),
            Err(err) => log::error!("could not {action} for character {character_id}: {err}"),
        }
    }

    // Callbacks run on the UI thread, so the window can be updated directly.
    fn show(&self, data: &BuilderData, character_id: Uuid, open_page: bool) {
        let Some(ui) = self.ui_weak.upgrade() else {
            return;
        };
        let Some(character) = data
            .adventure
            .characters
            .iter()
            .find(|character| character.id == character_id)
        else {
            log::error!("character {character_id} is not part of the adventure");
            return;
        };

        // The builder's cast cards show the same character, so keep them in step.
        show_builder(&self.ui_weak, data, false);
        ui.global::<CharacterGlobals>()
            .set_character(viewmodels::character_view(character));
        if open_page {
            ui.set_current_page("character".into());
        }
    }

    fn shown_adventure_id(&self) -> Option<Uuid> {
        let ui = self.ui_weak.upgrade()?;
        let character = ui.global::<CharacterGlobals>().get_character();
        parse_id(character.adventure_id.as_str())
    }

    /// Applies `change` to the shown character and saves it.
    fn edit_shown(
        &self,
        action: &str,
        character_id: &str,
        change: impl FnOnce(&mut Character) -> Result<(), CharacterPageError>,
    ) {
        let (Some(character_id), Some(adventure_id)) =
            (parse_id(character_id), self.shown_adventure_id())
        else {
            return;
        };

        self.run(
            action,
            adventure_id,
            character_id,
            false,
            self.apply(adventure_id, character_id, change),
        );
    }

    async fn apply(
        &self,
        adventure_id: Uuid,
        character_id: Uuid,
        change: impl FnOnce(&mut Character) -> Result<(), CharacterPageError>,
    ) -> Result<(), CharacterPageError> {
        let mut character = self
            .adventures
            .get_adventure(adventure_id)
            .await?
            .characters
            .into_iter()
            .find(|character| character.id == character_id)
            .ok_or(CharacterPageError::Invalid("character"))?;

        change(&mut character)?;
        self.characters.update_character(character).await?;
        Ok(())
    }
}

pub fn register_character_page<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
    A: ActCloser + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M, A>,
) {
    let env = PageEnv {
        handle: Handle::current(),
        adventures: ctx.adventures.clone(),
        characters: ctx.characters.clone(),
        ui_weak: ui.as_weak(),
    };

    register_open(ui, &env);
    register_edit(ui, &env);
}

/// Builder -> character page: open an existing character, or start a new one.
fn register_open(ui: &RootWindow, env: &PageEnv) {
    let builder = ui.global::<BuilderGlobals>();

    let open_env = env.clone();
    builder.on_character_selected(move |adventure_id, character_id| {
        let (Some(adventure_id), Some(character_id)) = (
            parse_id(adventure_id.as_str()),
            parse_id(character_id.as_str()),
        ) else {
            return;
        };

        open_env.run("open character", adventure_id, character_id, true, async {
            Ok(())
        });
    });

    let add_env = env.clone();
    builder.on_add_character(move |adventure_id| {
        let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
            return;
        };

        let character = Character::new(
            Uuid::new_v4(),
            NEW_CHARACTER_NAME.to_string(),
            adventure_id,
            "",
            "",
            CharacterStatus::Present,
            false,
        );
        let character_id = character.id;

        add_env.run("add character", adventure_id, character_id, true, async {
            add_env.characters.create_character(character).await?;
            Ok(())
        });
    });
}

/// Character page: each field and list item is saved on its own.
fn register_edit(ui: &RootWindow, env: &PageEnv) {
    let page = ui.global::<CharacterGlobals>();

    let save_env = env.clone();
    page.on_save_character(move |input| {
        save_env.edit_shown("save character", input.id.as_str(), move |character| {
            let name = input.name.trim();
            if name.is_empty() {
                return Err(CharacterPageError::Invalid("character name"));
            }
            let appearance = input.appearance.trim();

            character.name = name.to_string();
            character.headline = input.headline.trim().to_string();
            character.backstory = input.backstory.trim().to_string();
            character.appearance = (!appearance.is_empty()).then(|| appearance.to_string());
            Ok(())
        });
    });

    let env_add_trait = env.clone();
    page.on_add_trait(move |character_id, text| {
        env_add_trait.edit_shown("add trait", character_id.as_str(), move |character| {
            let text = text.trim();
            if text.is_empty() {
                return Err(CharacterPageError::Invalid("trait"));
            }
            character.add_trait(text.to_string());
            Ok(())
        });
    });

    let env_remove_trait = env.clone();
    page.on_remove_trait(move |character_id, index| {
        env_remove_trait.edit_shown("remove trait", character_id.as_str(), move |character| {
            remove_at(&mut character.traits, index, "trait")
        });
    });

    let env_add_tag = env.clone();
    page.on_add_tag(move |character_id, text| {
        env_add_tag.edit_shown("add tag", character_id.as_str(), move |character| {
            let text = text.trim();
            if text.is_empty() {
                return Err(CharacterPageError::Invalid("tag"));
            }
            character.add_tag(text.to_string());
            Ok(())
        });
    });

    let env_remove_tag = env.clone();
    page.on_remove_tag(move |character_id, index| {
        env_remove_tag.edit_shown("remove tag", character_id.as_str(), move |character| {
            remove_at(&mut character.tags, index, "tag")
        });
    });
}

fn remove_at(
    items: &mut Vec<String>,
    index: i32,
    what: &'static str,
) -> Result<(), CharacterPageError> {
    match usize::try_from(index) {
        Ok(index) if index < items.len() => {
            items.remove(index);
            Ok(())
        }
        _ => Err(CharacterPageError::Invalid(what)),
    }
}
