use living_voices::backend::domain::adventure::Adventure;

fn new_adventure() -> Adventure {
    Adventure::new("Tides of Glass".to_string(), "A drowned city".to_string(), String::from("NARRATOR"))
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
    assert!(adventure.tone.is_none());
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
