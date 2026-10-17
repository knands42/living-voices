use living_voices::backend::application::state::{AppState, SharedState, StateError};
use living_voices::backend::domain::adventure::Adventure;

fn new_adventure() -> Adventure {
    Adventure::new(
        "Tides of Glass".to_string(),
        "A drowned city".to_string(),
        String::from("NARRATOR"),
    )
}

#[test]
fn app_state_starts_without_adventure() {
    let state = AppState::new();

    assert!(state.current_adventure.is_none());
}

#[test]
fn app_state_set_adventure_stores_adventure() {
    let mut state = AppState::new();
    let adventure = new_adventure();

    state.set_adventure(adventure.clone());

    assert_eq!(
        state.current_adventure.as_ref().map(|a| a.id),
        Some(adventure.id)
    );
}

#[test]
fn set_adventure_replaces_previous_adventure() {
    let mut state = AppState::new();
    let first = new_adventure();
    let second = new_adventure();

    state.set_adventure(first);
    state.set_adventure(second.clone());

    assert_eq!(
        state.current_adventure.as_ref().map(|a| a.id),
        Some(second.id)
    );
}

#[test]
fn shared_state_starts_without_adventure() {
    let state = SharedState::new();

    assert!(state.read().unwrap().current_adventure.is_none());
}

#[test]
fn shared_state_exposes_written_adventure_to_later_reads() {
    let state = SharedState::new();
    let adventure = new_adventure();

    state.write().unwrap().set_adventure(adventure.clone());

    assert_eq!(
        state
            .read()
            .unwrap()
            .current_adventure
            .as_ref()
            .map(|a| a.id),
        Some(adventure.id)
    );
}

#[test]
fn shared_state_shares_one_state_across_clones() {
    let state = SharedState::new();
    let clone = state.clone();
    let adventure = new_adventure();

    state.write().unwrap().set_adventure(adventure.clone());

    assert_eq!(
        clone
            .read()
            .unwrap()
            .current_adventure
            .as_ref()
            .map(|a| a.id),
        Some(adventure.id)
    );
}

#[test]
fn read_fails_while_write_guard_is_held() {
    let state = SharedState::new();
    let _guard = state.write().unwrap();

    assert!(matches!(state.read(), Err(StateError::ReadLockFailed(_))));
}

#[test]
fn write_fails_while_another_write_guard_is_held() {
    let state = SharedState::new();
    let _guard = state.write().unwrap();

    assert!(matches!(state.write(), Err(StateError::WriteLockFailed(_))));
}

#[test]
fn default_matches_new() {
    let state = SharedState::default();

    assert!(state.read().unwrap().current_adventure.is_none());
}
