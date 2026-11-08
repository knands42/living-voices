use chrono::Utc;
use living_voices::backend::domain::adventure::{Adventure, AdventureStatus};
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::domain::lore::Lore;
use living_voices::backend::domain::narrator::Narrator;
use uuid::Uuid;

fn new_adventure() -> Adventure {
    let now = Utc::now();
    Adventure::new(
        Uuid::new_v4(),
        "Tides of Glass",
        "A drowned city",
        Narrator::new(Uuid::new_v4(), Uuid::new_v4(), "NARRATOR", ""),
        "",
        Lore::new(Uuid::new_v4(), ""),
        AdventureStatus::Draft,
        now,
        now,
    )
}

fn adventure_with_character(name: &str) -> (Adventure, Character) {
    let mut adventure = new_adventure();
    let character = Character::new(
        Uuid::new_v4(),
        name.to_string(),
        "female".to_string(),
        adventure.id,
        "A diver",
        "",
        CharacterStatus::Present,
        false,
    );
    adventure.add_character(character.clone());
    (adventure, character)
}

#[test]
fn new_sets_title_description_and_empty_collections() {
    let adventure = new_adventure();

    assert_eq!(adventure.title, "Tides of Glass");
    assert_eq!(adventure.headline, "A drowned city");
    assert!(adventure.characters.is_empty());
    assert!(adventure.tags.is_empty());
    assert_eq!(adventure.lore.text, "");
}

#[test]
fn new_defaults_tone_to_empty() {
    let adventure = new_adventure();

    assert!(adventure.tone.is_empty());
}

#[test]
fn status_round_trips_through_its_string_form() {
    for status in [
        AdventureStatus::Draft,
        AdventureStatus::Private,
        AdventureStatus::Public,
    ] {
        assert_eq!(AdventureStatus::from_str(status.as_str()), status);
    }
}

#[test]
fn status_defaults_to_draft_for_unknown_values() {
    assert_eq!(
        AdventureStatus::from_str("nonsense"),
        AdventureStatus::Draft
    );
    assert_eq!(AdventureStatus::default(), AdventureStatus::Draft);
}

#[test]
fn new_stamps_matching_timestamps() {
    let adventure = new_adventure();

    assert_eq!(adventure.created_at, adventure.updated_at);
}

#[test]
fn new_generates_unique_ids() {
    let first = new_adventure();
    let second = new_adventure();

    assert_ne!(first.id, second.id);
}

#[test]
fn add_tags_appends_to_existing_tags() {
    let mut adventure = new_adventure();
    adventure.add_tags(vec!["coastal".to_string()]);
    adventure.add_tags(vec!["mystery".to_string(), "tragedy".to_string()]);

    assert_eq!(adventure.tags, vec!["coastal", "mystery", "tragedy"]);
}

#[test]
fn find_character_returns_matching_character() {
    // Given an adventure holding a character named "Mira"
    let (adventure, mira) = adventure_with_character("Mira");

    // When looking the character up by name
    let found = adventure.find_character(mira.id);

    // Then the stored character is returned
    assert!(found.is_some());
    assert_eq!(found.expect("found").id, mira.id);
}

#[test]
fn find_character_returns_none_for_unknown_name() {
    // Given an adventure holding a character named "Mira"
    let (adventure, _mira) = adventure_with_character("Mira");

    // When looking up a name that is not in the cast
    // Then nothing is returned
    assert!(adventure.find_character(Uuid::new_v4()).is_none());
}

#[test]
fn active_characters_returns_only_present_names() {
    // Given an adventure with three cast members
    let mut adventure = new_adventure();
    for name in ["Mira", "Kael", "Otho"] {
        let character = Character::new(
            Uuid::new_v4(),
            name.to_string(),
            "female".to_string(),
            adventure.id,
            "A diver",
            "",
            CharacterStatus::Present,
            false,
        );
        adventure.add_character(character);
    }

    // When filtering to two present names
    let present = vec!["Mira".to_string(), "Otho".to_string()];
    let active = adventure.active_characters(&present);

    // Then only the present cast members come back
    let names: Vec<&str> = active.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Mira", "Otho"]);
}

#[test]
fn active_characters_ignores_names_not_in_the_cast() {
    // Given an adventure with one cast member
    let (adventure, _mira) = adventure_with_character("Mira");

    // When a present name is not in the cast
    let active = adventure.active_characters(&["Ghost".to_string()]);

    // Then no character is returned
    assert!(active.is_empty());
}

#[test]
fn message_from_reply_builds_a_character_say_when_replier_is_in_the_cast() {
    // Given an adventure whose cast includes "Mira"
    let (adventure, mira) = adventure_with_character("Mira");

    // When building a reply message attributed to "Mira"
    let message = adventure.message_from_reply("Mira", "The tide is turning.");

    // Then it is attributed to that character, carrying the character id and no narrator id
    assert_eq!(message.adventure_id, adventure.id);
    assert_eq!(message.character_id, Some(mira.id));
    assert!(message.narrator_id.is_none());
    assert_eq!(message.speaker_name, "Mira");
    assert_eq!(message.text, "The tide is turning.");
    assert!(!message.ooc);
}

#[test]
fn message_from_reply_falls_through_to_the_narrator_for_an_unknown_replier() {
    // Given an adventure whose cast does not include "Ghost"
    let (adventure, _mira) = adventure_with_character("Mira");

    // When building a reply message attributed to a name outside the cast
    let message = adventure.message_from_reply("Ghost", "The city holds its breath.");

    // Then it is spoken by the narrator, carrying the narrator id and no character id
    assert!(message.character_id.is_none());
    assert_eq!(message.narrator_id, Some(adventure.narrator.id));
    assert_eq!(message.speaker_name, "NARRATOR");
    assert_eq!(message.text, "The city holds its breath.");
}

#[test]
fn message_from_reply_strips_a_character_header_before_matching() {
    // Given an adventure whose cast includes "Mira"
    let (adventure, mira) = adventure_with_character("Mira");

    // When the replier arrives wrapped in the model's header noise
    let message = adventure.message_from_reply("[CHARACTER] Mira", "The tide is turning.");

    // Then it still resolves to the character rather than collapsing to the narrator
    assert_eq!(message.character_id, Some(mira.id));
    assert!(message.narrator_id.is_none());
    assert_eq!(message.speaker_name, "Mira");
}

#[test]
fn message_from_reply_drops_the_speaker_name_echoed_into_the_body() {
    // Given a cast with "Mira"
    let (adventure, _mira) = adventure_with_character("Mira");

    // When the model echoes the speaker into the reply body
    let message = adventure.message_from_reply("Mira", "Mira: The tide is turning.");

    // Then only the body survives; the speaker lives in speaker_name
    assert_eq!(message.text, "The tide is turning.");
    assert_eq!(message.speaker_name, "Mira");
}

#[test]
fn message_from_reply_keeps_a_colon_that_is_not_the_speaker_name() {
    let (adventure, _mira) = adventure_with_character("Mira");

    // A line that merely contains a colon is left untouched
    let message = adventure.message_from_reply("Mira", "Note: the map is wrong.");

    assert_eq!(message.text, "Note: the map is wrong.");
}

#[test]
fn message_from_reply_drops_the_narrator_name_echoed_into_the_body() {
    let (adventure, _mira) = adventure_with_character("Mira");

    let message = adventure.message_from_reply("NARRATOR", "NARRATOR: A horn sounds.");

    assert_eq!(message.text, "A horn sounds.");
    assert_eq!(message.speaker_name, "NARRATOR");
}

#[test]
fn player_name_returns_the_is_player_characters_name() {
    let mut adventure = new_adventure();
    let player = Character::new(
        Uuid::new_v4(),
        "You".to_string(),
        "female".to_string(),
        adventure.id,
        "the player",
        "",
        CharacterStatus::Present,
        true,
    );
    adventure.add_character(player);

    assert_eq!(adventure.player_name(), Some("You"));
}

#[test]
fn message_from_reply_strips_a_trailing_target_qualifier() {
    let (adventure, mira) = adventure_with_character("Mira");

    let message = adventure.message_from_reply("[CHARACTER] Mira | target: Kael", "Over here.");

    assert_eq!(message.character_id, Some(mira.id));
    assert_eq!(message.speaker_name, "Mira");
}

#[test]
fn message_from_reply_matches_a_character_case_insensitively() {
    let (adventure, mira) = adventure_with_character("Mira");

    let message = adventure.message_from_reply("  mira  ", "The gate is shut.");

    assert_eq!(message.character_id, Some(mira.id));
    assert_eq!(message.speaker_name, "Mira");
}

#[test]
fn message_from_reply_routes_an_explicit_narrator_header_to_the_narrator() {
    // Given an adventure whose cast includes "Mira"
    let (adventure, _mira) = adventure_with_character("Mira");

    // When the reply is explicitly the narrator's
    let message = adventure.message_from_reply("[NARRATOR]", "Torches flare along the wall.");

    // Then it is attributed to the narrator
    assert!(message.character_id.is_none());
    assert_eq!(message.narrator_id, Some(adventure.narrator.id));
    assert_eq!(message.speaker_name, "NARRATOR");
}

#[test]
fn message_from_reply_routes_a_bare_narrator_name_to_the_narrator() {
    let (adventure, _mira) = adventure_with_character("Mira");

    let message = adventure.message_from_reply("NARRATOR", "The sea keeps its own ledger.");

    assert!(message.character_id.is_none());
    assert_eq!(message.narrator_id, Some(adventure.narrator.id));
}
