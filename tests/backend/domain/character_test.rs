use living_voices::backend::domain::character::{Character, CharacterStatus};
use uuid::Uuid;

fn new_character() -> Character {
    Character::new("Maren".to_string(), None, "A lighthouse keeper".to_string())
}

#[test]
fn new_sets_name_description_and_empty_fields() {
    let character = new_character();

    assert_eq!(character.name, "Maren");
    assert_eq!(character.description, "A lighthouse keeper");
    assert_eq!(character.backstory, "");
    assert!(character.traits.is_empty());
    assert!(character.tags.is_empty());
    assert!(character.relations.is_empty());
}

#[test]
fn new_defaults_status_to_active() {
    let character = new_character();

    assert_eq!(character.status, CharacterStatus::default());
    assert_eq!(CharacterStatus::default().as_str(), "present");
}

#[test]
fn with_backstory_replaces_backstory() {
    let character =
        new_character().with_backstory("She kept the lamp lit for forty years.".to_string());

    assert_eq!(
        character.backstory,
        "She kept the lamp lit for forty years."
    );
}

#[test]
fn with_status_replaces_status() {
    let character = new_character().with_status(CharacterStatus::Present);

    assert_eq!(character.status, CharacterStatus::Present);
}

#[test]
fn add_tag_appends_tag() {
    let mut character = new_character();
    character.add_tags("protagonist".to_string());
    character.add_tags("brave".to_string());

    assert_eq!(character.tags, vec!["protagonist", "brave"]);
}

#[test]
fn remove_tag_drops_every_matching_entry() {
    let mut character = new_character();
    character.add_tags("brave".to_string());
    character.add_tags("brave".to_string());
    character.add_tags("lonely".to_string());

    character.remove_tag("brave".to_string());

    assert_eq!(character.tags, vec!["lonely"]);
}

#[test]
fn add_trait_appends_trait() {
    let mut character = new_character();
    character.add_trait("speaks only in tidal registers".to_string());

    assert_eq!(character.traits, vec!["speaks only in tidal registers"]);
}

#[test]
fn add_relation_maps_entity_to_relationship() {
    let mut character = new_character();
    let mother = Uuid::new_v4();

    character.add_relation(mother, "Mother".to_string());

    assert_eq!(
        character.relations.get(&mother),
        Some(&"Mother".to_string())
    );
}

#[test]
fn add_relation_replaces_existing_relationship_for_same_entity() {
    let mut character = new_character();
    let sister = Uuid::new_v4();

    character.add_relation(sister, "Sister".to_string());
    character.add_relation(sister, "Estranged sister".to_string());

    assert_eq!(character.relations.len(), 1);
    assert_eq!(
        character.relations.get(&sister),
        Some(&"Estranged sister".to_string())
    );
}

#[test]
fn new_generates_unique_ids() {
    assert_ne!(new_character().id, new_character().id);
}
