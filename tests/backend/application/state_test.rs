use chrono::Utc;
use living_voices::backend::application::state::{AppState, SharedState, StateError};
use living_voices::backend::domain::adventure::{Adventure, AdventureStatus};
use living_voices::backend::domain::lore::Lore;
use living_voices::backend::domain::narrator::Narrator;
use uuid::Uuid;

fn new_adventure() -> Adventure {
    Adventure::new(
        Uuid::new_v4(),
        "Tides of Glass",
        "A drowned city",
        Narrator::new(Uuid::new_v4(), Uuid::new_v4(), "NARRATOR", ""),
        "",
        Lore::new(Uuid::new_v4(), ""),
        AdventureStatus::Draft,
        Utc::now(),
        Utc::now(),
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

    state.current_adventure = Some(adventure.clone());

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

    state.current_adventure = Some(first);
    state.current_adventure = Some(second.clone());

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

    state.write().unwrap().current_adventure = Some(adventure.clone());

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

    state.write().unwrap().current_adventure = Some(adventure.clone());

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
