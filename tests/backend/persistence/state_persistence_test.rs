use crate::backend::application::helper::setup_db;
use crate::backend::persistence::helper::seed_adventure;
use living_voices::backend::domain::memory::State;
use living_voices::backend::persistence::state_persistence::{
    get_active_candidates, get_state, upsert_state,
};

#[tokio::test]
async fn upsert_state_inserts_then_updates_a_single_row() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // First upsert inserts.
    let mut state = State::new(adventure.id);
    state.day = "Day 1".into();
    state.place = "the shore".into();
    state.characters_present = vec!["Mira".into()];
    upsert_state(&mut conn, &state).await.expect("insert state");

    let loaded = get_state(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("some state");
    assert_eq!(loaded.day, "Day 1");
    assert_eq!(loaded.place, "the shore");
    assert_eq!(loaded.characters_present, vec!["Mira".to_string()]);

    // Second upsert on the same adventure takes the ON CONFLICT update path.
    let mut next = State::new(adventure.id);
    next.day = "Day 2".into();
    next.characters_present = vec!["Mira".into(), "Kael".into()];
    upsert_state(&mut conn, &next).await.expect("update state");

    let loaded = get_state(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("some state");
    assert_eq!(loaded.day, "Day 2");
    assert_eq!(loaded.place, "unknown");
    assert_eq!(
        loaded.characters_present,
        vec!["Mira".to_string(), "Kael".to_string()]
    );

    // The adventure still has exactly one state row.
    let count: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM state WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn get_state_is_none_before_any_upsert() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    assert!(
        get_state(&mut conn, adventure.id)
            .await
            .expect("get")
            .is_none()
    );
}

#[tokio::test]
async fn get_active_candidates_reads_characters_present() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let mut state = State::new(adventure.id);
    state.characters_present = vec!["Mira".into(), "Kael".into()];
    upsert_state(&mut conn, &state).await.expect("upsert");

    let names = get_active_candidates(&mut conn, &adventure.id.to_string())
        .await
        .expect("candidates");
    assert_eq!(names, vec!["Mira".to_string(), "Kael".to_string()]);
}
