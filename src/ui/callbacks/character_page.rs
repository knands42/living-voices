use crate::app::DefaultContext;
use crate::backend::application::adventure_manager::AdventureManager;
use crate::backend::application::character_manager::{CharacterManager, CharacterManagerError};
use crate::backend::application::port::chat_port::PromptPort;
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::domain::character::Character;
use crate::ui::callbacks::parse_id;
use crate::ui::{BuilderGlobals, ChangedField, CharacterGlobals, RootWindow, notice, viewmodels};
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
struct CharacterPageContext<P: PromptPort + Clone + 'static, I: ImagePort + Clone + 'static> {
    handle: Handle,
    adventures: AdventureManager,
    characters: CharacterManager<P, I>,
    ui_weak: slint::Weak<RootWindow>,
}

impl<P: PromptPort + Clone + 'static, I: ImagePort + Clone + 'static> CharacterPageContext<P, I> {
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

    /// Off the UI thread, the one way back onto it: the flags are cleared on the UI loop
    /// and the character it shows is re-read from the snapshot the run just updated.
    fn edit_ui(&self, update: impl FnOnce(&CharacterGlobals) + Send + 'static) {
        let env = self.clone();
        let post = slint::invoke_from_event_loop(move || {
            let Some(ui) = env.ui_weak.upgrade() else {
                return;
            };
            let page = ui.global::<CharacterGlobals>();

            update(&page);

            let shown = parse_id(page.get_character().id.as_str());
            if let Some(character) = shown.and_then(|id| env.character(id)) {
                page.set_character(viewmodels::character_view(&character));
            }
        });

        if let Err(err) = post {
            log::error!("could not post a character update to the UI: {err}");
        }
    }

    fn spawn_appearance(&self, character_id: Uuid) {
        let env = self.clone();

        if let Some(ui) = env.ui_weak.upgrade() {
            ui.global::<CharacterGlobals>().set_generating_prompts(true);
        }

        self.handle.spawn(async move {
            let Some(character) = env.character(character_id) else {
                notice::error(
                    &env.ui_weak,
                    format!("character {character_id} vanished before its prompts ran"),
                );
                env.edit_ui(|page| {
                    page.set_generating_prompts(false);
                    page.set_generating_image(false);
                });
                return;
            };

            let (identity, outfit) = tokio::join!(
                env.characters.create_character_identity_prompt(&character),
                env.characters.create_character_outfit_prompt(&character),
            );

            if let Err(err) = identity {
                notice::error(
                    &env.ui_weak,
                    format!("could not generate the identity prompt of {character_id}: {err}"),
                );
            }
            if let Err(err) = outfit {
                notice::error(
                    &env.ui_weak,
                    format!("could not generate the outfit prompt of {character_id}: {err}"),
                );
            }

            env.post_prompts_ready();

            if let Some(appearance) = character.appearance {
                if appearance.is_empty() {
                    env.post_portrait_being_generated();
                }

                match env.character(character_id) {
                    Some(character) => {
                        if let Err(err) = env
                            .characters
                            .create_character_image_portrait(&character)
                            .await
                        {
                            notice::warn(
                                &env.ui_weak,
                                format!(
                                    "could not generate the image portrait for {0} in the last 30 seconds: {err}\nIf the image does not show up in a few seconds, try again later", character.name
                                ),
                            );
                        }
                    }
                    None => notice::error(
                        &env.ui_weak,
                        format!("character {character_id} vanished while its prompts ran"),
                    ),
                }

                env.post_portrait_ready();
            }
        });
    }

    /// The prompts for appearance landed
    fn post_prompts_ready(&self) {
        self.edit_ui(|page| {
            page.set_generating_prompts(false);
        });
    }

    fn post_portrait_being_generated(&self) {
        self.edit_ui(|page| {
            page.set_generating_image(true);
        });
    }

    /// The image portrait landed
    fn post_portrait_ready(&self) {
        self.edit_ui(|page| {
            page.set_generating_image(false);
        });
    }

    fn character(&self, character_id: Uuid) -> Option<Character> {
        self.adventures.current_adventure().and_then(|adventure| {
            adventure
                .characters
                .into_iter()
                .find(|character| character.id == character_id)
        })
    }
}

pub fn register_character_page(ui: &RootWindow, ctx: &DefaultContext) {
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
fn register_open<P: PromptPort + Clone + 'static, I: ImagePort + Clone + 'static>(
    ui: &RootWindow,
    env: &CharacterPageContext<P, I>,
) {
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
            None => notice::error(
                &open_env.ui_weak,
                format!("character {character_id} is not part of the adventure"),
            ),
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
            Err(err) => notice::error(
                &add_env.ui_weak,
                format!("could not create a character in {adventure_id}: {err}"),
            ),
        }
    });
}

/// Character page: each field and list item is saved on its own.
fn register_edit<P: PromptPort + Clone + 'static, I: ImagePort + Clone + 'static>(
    ui: &RootWindow,
    env: &CharacterPageContext<P, I>,
) {
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
            notice::error(
                &save_env.ui_weak,
                format!("could not save character {character_id}: {err}"),
            );
            return;
        }

        if changed_field == ChangedField::Appearance {
            save_env.spawn_appearance(character_id);
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
            notice::error(
                &env_add_trait.ui_weak,
                format!("could not add a trait to character {character_id}: {err}"),
            );
        }
    });

    let env_remove_trait = env.clone();
    page.on_remove_trait(move |character_id, index| {
        let result = env_remove_trait.edit_character(character_id.as_str(), move |character| {
            remove_at(&mut character.traits, index, "trait")
        });

        if let Err(err) = result {
            notice::error(
                &env_remove_trait.ui_weak,
                format!("could not remove a trait from character {character_id}: {err}"),
            );
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
            notice::error(
                &env_add_tag.ui_weak,
                format!("could not add a tag to character {character_id}: {err}"),
            );
        }
    });

    let env_remove_tag = env.clone();
    page.on_remove_tag(move |character_id, index| {
        let result = env_remove_tag.edit_character(character_id.as_str(), move |character| {
            remove_at(&mut character.tags, index, "tag")
        });

        if let Err(err) = result {
            notice::error(
                &env_remove_tag.ui_weak,
                format!("could not remove a tag from character {character_id}: {err}"),
            );
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
            notice::error(
                &env_add_relation.ui_weak,
                format!("could not add a relation to character {character_id}: {err}"),
            );
        }
    });

    let env_remove_relation = env.clone();
    page.on_remove_relation(move |character_id, index| {
        let result = env_remove_relation.edit_character(character_id.as_str(), move |character| {
            remove_at(&mut character.relations, index, "relation")
        });

        if let Err(err) = result {
            notice::error(
                &env_remove_relation.ui_weak,
                format!("could not remove a relation from character {character_id}: {err}"),
            );
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
