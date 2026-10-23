use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::Character;

fn new_adventure() -> Adventure {
    Adventure::new(
        "Tides of Glass".to_string(),
        "A drowned city".to_string(),
        String::from("NARRATOR"),
        "",
    )
}

fn adventure_with_character(name: &str) -> (Adventure, Character) {
    let mut adventure = new_adventure();
    let character = Character::new(name.to_string(), Some(adventure.id), "A diver".to_string());
    adventure.characters.push(character.clone());
    (adventure, character)
}

#[test]
fn new_sets_title_description_and_empty_collections() {
    let adventure = new_adventure();

    assert_eq!(adventure.title, "Tides of Glass");
    assert_eq!(adventure.description, "A drowned city");
    assert!(adventure.characters.is_empty());
    assert!(adventure.tags.is_empty());
    assert_eq!(adventure.lore.text, "");
}

#[test]
fn new_defaults_optional_metadata_to_none() {
    let adventure = new_adventure();

    assert!(adventure.genre.is_none());
    assert!(adventure.tone.is_empty());
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
    let found = adventure.find_character("Mira");

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
    assert!(adventure.find_character("Nobody").is_none());
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
