use living_voices::backend::application::{adventure_manager::AdventureManager, state::SharedState};

fn new_manager() -> AdventureManager {
    AdventureManager::new(SharedState::new())
}

#[test]
fn create_adventure_returns_adventure_with_given_fields() {
    let mut manager = new_manager();

    let adventure = manager
        .create_adventure("Tides of Glass".to_string(), "A drowned city".to_string())
        .unwrap();

    assert_eq!(adventure.title, "Tides of Glass");
    assert_eq!(adventure.description, "A drowned city");
}

#[test]
fn create_adventure_stores_adventure_in_state() {
    let mut manager = new_manager();

    let adventure = manager
        .create_adventure("Tides of Glass".to_string(), "A drowned city".to_string())
        .unwrap();

    let stored = manager.state().read().unwrap().current_adventure.clone();
    assert_eq!(stored.map(|a| a.id), Some(adventure.id));
}

#[test]
fn create_adventure_replaces_previous_adventure_in_state() {
    let mut manager = new_manager();

    let first = manager.create_adventure("First".to_string(), "One".to_string()).unwrap();
    let second = manager.create_adventure("Second".to_string(), "Two".to_string()).unwrap();

    let stored = manager.state().read().unwrap().current_adventure.clone();
    assert_eq!(stored.map(|a| a.id), Some(second.id));
    assert_ne!(first.id, second.id);
}

#[test]
fn create_adventure_generates_unique_ids_across_calls() {
    let mut manager = new_manager();

    let first = manager.create_adventure("First".to_string(), "One".to_string()).unwrap();
    let second = manager.create_adventure("Second".to_string(), "Two".to_string()).unwrap();

    assert_ne!(first.id, second.id);
}

#[test]
fn state_is_shared_with_the_state_passed_to_new() {
    let state = SharedState::new();
    let mut manager = AdventureManager::new(state.clone());

    let adventure = manager.create_adventure("Tides of Glass".to_string(), "Drowned".to_string()).unwrap();

    assert_eq!(state.read().unwrap().current_adventure.as_ref().map(|a| a.id), Some(adventure.id));
}

#[test]
fn create_adventure_leaves_stored_adventure_in_shared_state() {
    let mut manager = new_manager();

    let created = manager.create_adventure("Tides of Glass".to_string(), "A drowned city".to_string()).unwrap();

    assert_eq!(created.lore.text, "");
    assert!(created.characters.is_empty());
}
