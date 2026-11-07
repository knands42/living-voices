use crate::ai::integrations::text::chat::Chat;
use crate::app::AppContext;
use crate::backend::application::adventure_manager::AdventureManager;
use crate::backend::application::character_manager::{CharacterManager, CharacterManagerError};
use crate::backend::application::port::chat_port::{ChatHandler, NarratorHandler};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::domain::character::Character;
use crate::ui::callbacks::adventure_creation::parse_id;
use crate::ui::{BuilderGlobals, ChangedField, CharacterGlobals, RootWindow, viewmodels};
use slint::ComponentHandle;
use thiserror::Error;
use tokio::runtime::Handle;
use uuid::Uuid;

#[derive(Error, Debug)]
enum CharacterPageError {
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Character(#[from] CharacterManagerError),
}

/// Everything a character-page callback needs, cheap to clone into each closure.
///
/// Edits are reasoned about in three steps and nothing else: find the character in the
/// snapshot, change it, write it back. The manager keeps the snapshot current on save,
/// so the builder is refreshed when its page is entered, never in the middle of an edit.
#[derive(Clone)]
struct CharacterPageContext {
    handle: Handle,
    adventures: AdventureManager,
    characters: CharacterManager<Chat>,
    ui_weak: slint::Weak<RootWindow>,
}

impl CharacterPageContext {
    // Callbacks run on the UI thread, so the window can be updated directly.
    fn show(&self, character: &Character) {
        let Some(ui) = self.ui_weak.upgrade() else {
            return;
        };

        ui.global::<CharacterGlobals>()
            .set_character(viewmodels::character_view(character));
    }

    fn open(&self, character: &Character) {
        self.show(character);
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_current_page("character".into());
        }
    }

    /// The character the page shows, taken from the backend snapshot.
    fn current_character(&self) -> Option<Character> {
        let character_id = self.shown_id()?;

        self.adventures
            .current_adventure()?
            .characters
            .into_iter()
            .find(|character| character.id == character_id)
    }

    fn shown_id(&self) -> Option<Uuid> {
        let ui = self.ui_weak.upgrade()?;
        parse_id(ui.global::<CharacterGlobals>().get_character().id.as_str())
    }

    /// Finds the character in the snapshot, applies `change`, writes it once
    fn edit_character(
        &self,
        character_id: &str,
        change: impl FnOnce(&mut Character) -> Result<(), CharacterPageError>,
    ) -> Result<(), CharacterPageError> {
        let character_id =
            parse_id(character_id).ok_or(CharacterPageError::Invalid("character id"))?;
        let mut character = self
            .adventures
            .current_adventure()
            .and_then(|adventure| {
                adventure
                    .characters
                    .into_iter()
                    .find(|character| character.id == character_id)
            })
            .ok_or(CharacterPageError::Invalid("character"))?;

        change(&mut character)?;

        let updated = character.clone();
        tokio::task::block_in_place(|| {
            self.handle
                .block_on(self.characters.update_character(updated))
        })?;

        self.show(&character);
        Ok(())
    }

    /// The appearance prose is what the two image layers are distilled from, so every save
    /// of the field rebuilds both of them in the background. The field stays dimmed while
    /// that runs, which is the only signal the player gets that a model call is in flight.
    fn start_image_prompts(&self, character_id: Uuid) {
        let Some(character) = self.character(character_id) else {
            return;
        };
        if character.is_placeholder() || character.appearance.is_none() {
            return;
        }

        self.spawn_appearance_prompts(character);
    }

    fn character(&self, character_id: Uuid) -> Option<Character> {
        self.adventures.current_adventure().and_then(|adventure| {
            adventure
                .characters
                .into_iter()
                .find(|character| character.id == character_id)
        })
    }

    fn spawn_appearance_prompts(&self, character: Character) {
        let character_id = character.id;
        let characters = self.characters.clone();
        let adventures = self.adventures.clone();
        let ui_weak = self.ui_weak.clone();

        if let Some(ui) = ui_weak.upgrade() {
            ui.global::<CharacterGlobals>().set_generating_prompts(true);
        }

        self.handle.spawn(async move {
            let (identity, outfit) = tokio::join!(
                characters.create_character_identity_prompt(&character),
                characters.create_character_outfit_prompt(&character),
            );

            if let Err(err) = identity {
                log::error!("could not generate the identity prompt of {character_id}: {err}");
            }
            if let Err(err) = outfit {
                log::error!("could not generate the outfit prompt of {character_id}: {err}");
            }

            let update_post = slint::invoke_from_event_loop(move || {
                let Some(ui) = ui_weak.upgrade() else {
                    return;
                };
                let character_page = ui.global::<CharacterGlobals>();
                character_page.set_generating_prompts(false);

                if parse_id(character_page.get_character().id.as_str()) != Some(character_id) {
                    return;
                }

                let refreshed = adventures.current_adventure().and_then(|adventure| {
                    adventure
                        .characters
                        .into_iter()
                        .find(|character| character.id == character_id)
                });

                if let Some(character) = refreshed {
                    character_page.set_character(viewmodels::character_view(&character));
                }
            });

            if let Err(err) = update_post {
                log::error!("could not re-render character {character_id}: {err}");
            }
        });
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
    let env = CharacterPageContext {
        handle: Handle::current(),
        adventures: ctx.adventures.clone(),
        characters: ctx.characters.clone(),
        ui_weak: ui.as_weak(),
    };

    register_open(ui, &env);
    register_edit(ui, &env);
}

/// Builder -> character page: open an existing character, or start a new one.
fn register_open(ui: &RootWindow, env: &CharacterPageContext) {
    let builder = ui.global::<BuilderGlobals>();

    let open_env = env.clone();
    builder.on_character_selected(move |_adventure_id, character_id| {
        let Some(character_id) = parse_id(character_id.as_str()) else {
            return;
        };

        let character = open_env
            .adventures
            .current_adventure()
            .and_then(|adventure| {
                adventure
                    .characters
                    .into_iter()
                    .find(|character| character.id == character_id)
            });

        match character {
            Some(character) => open_env.open(&character),
            None => log::error!("character {character_id} is not part of the adventure"),
        }
    });

    let add_env = env.clone();
    builder.on_add_character(move |adventure_id| {
        let Some(adventure_id) = parse_id(adventure_id.as_str()) else {
            return;
        };

        let result = tokio::task::block_in_place(|| {
            add_env
                .handle
                .block_on(add_env.characters.create_draft(adventure_id))
        });

        match result {
            Ok(character) => add_env.open(&character),
            Err(err) => log::error!("could not create a character in {adventure_id}: {err}"),
        }
    });
}

/// Character page: each field and list item is saved on its own.
fn register_edit(ui: &RootWindow, env: &CharacterPageContext) {
    let page = ui.global::<CharacterGlobals>();

    let save_env = env.clone();
    page.on_save_character(move |input, changed_field| {
        let Some(character_id) = save_env.shown_id() else {
            return;
        };

        let result = save_env.edit_character(character_id.to_string().as_str(), move |character| {
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

        if let Err(err) = result {
            log::error!("could not save character {character_id}: {err}");
            return;
        }

        if changed_field == ChangedField::Appearance {
            save_env.start_image_prompts(character_id);
        }
    });

    let env_add_trait = env.clone();
    page.on_add_trait(move |character_id, text| {
        let result = env_add_trait.edit_character(character_id.as_str(), move |character| {
            let text = text.trim();
            if text.is_empty() {
                return Err(CharacterPageError::Invalid("trait"));
            }
            character.add_trait(text.to_string());
            Ok(())
        });

        if let Err(err) = result {
            log::error!("could not add a trait to character {character_id}: {err}");
        }
    });

    let env_remove_trait = env.clone();
    page.on_remove_trait(move |character_id, index| {
        let result = env_remove_trait.edit_character(character_id.as_str(), move |character| {
            remove_at(&mut character.traits, index, "trait")
        });

        if let Err(err) = result {
            log::error!("could not remove a trait from character {character_id}: {err}");
        }
    });

    let env_add_tag = env.clone();
    page.on_add_tag(move |character_id, text| {
        let result = env_add_tag.edit_character(character_id.as_str(), move |character| {
            let text = text.trim();
            if text.is_empty() {
                return Err(CharacterPageError::Invalid("tag"));
            }
            character.add_tag(text.to_string());
            Ok(())
        });

        if let Err(err) = result {
            log::error!("could not add a tag to character {character_id}: {err}");
        }
    });

    let env_remove_tag = env.clone();
    page.on_remove_tag(move |character_id, index| {
        let result = env_remove_tag.edit_character(character_id.as_str(), move |character| {
            remove_at(&mut character.tags, index, "tag")
        });

        if let Err(err) = result {
            log::error!("could not remove a tag from character {character_id}: {err}");
        }
    });

    let env_add_relation = env.clone();
    page.on_add_relation(move |character_id, text| {
        let result = env_add_relation.edit_character(character_id.as_str(), move |character| {
            let text = text.trim();
            if text.is_empty() {
                return Err(CharacterPageError::Invalid("relation"));
            }
            character.add_relation(text.to_string());
            Ok(())
        });

        if let Err(err) = result {
            log::error!("could not add a relation to character {character_id}: {err}");
        }
    });

    let env_remove_relation = env.clone();
    page.on_remove_relation(move |character_id, index| {
        let result = env_remove_relation.edit_character(character_id.as_str(), move |character| {
            remove_at(&mut character.relations, index, "relation")
        });

        if let Err(err) = result {
            log::error!("could not remove a relation from character {character_id}: {err}");
        }
    });

    let env_appearance = env.clone();
    page.on_add_appearance_prompt(move |character_id| {
        let Some(character_id) = parse_id(character_id.as_str()) else {
            return;
        };

        let result = tokio::task::block_in_place(|| {
            env_appearance.handle.block_on(async {
                let character = env_appearance
                    .adventures
                    .current_adventure()
                    .and_then(|adventure| {
                        adventure
                            .characters
                            .into_iter()
                            .find(|character| character.id == character_id)
                    })
                    .ok_or(CharacterPageError::Invalid("character"))?;

                env_appearance
                    .characters
                    .create_character_identity_prompt(&character)
                    .await?;
                Ok::<_, CharacterPageError>(())
            })
        });

        match result {
            // The manager wrote the new prompt into the snapshot, so re-read it there.
            Ok(()) => match env_appearance.current_character() {
                Some(character) => env_appearance.show(&character),
                None => log::error!("character {character_id} vanished while its prompt ran"),
            },
            Err(err) => {
                log::error!("could not create the appearance prompt of {character_id}: {err}")
            }
        }
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
