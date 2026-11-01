use uuid::Uuid;

use living_voices::backend::persistence::seeder::{SEED_ADVENTURES, seed_one};
use living_voices::backend::{open_pool, run_migrations};

struct TempDb(String);
impl Drop for TempDb {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0));
        }
    }
}

async fn fresh_db() -> TempDb {
    let path = std::env::temp_dir()
        .join(format!("living_voices_seeder_test_{}.db", Uuid::new_v4()))
        .to_string_lossy()
        .into_owned();
    let migrating = open_pool(&path).await.expect("open pool");
    run_migrations(&migrating).await.expect("run migrations");
    TempDb(path)
}

#[tokio::test]
async fn seed_one_populates_acts_state_facts_and_messages() {
    let db = fresh_db().await;
    let seed = &SEED_ADVENTURES[0];

    let adventure_id = seed_one(&db.0, seed.title).await.expect("seed one");

    let pool = open_pool(&db.0).await.expect("reopen pool");
    let mut conn = pool.acquire().await.expect("acquire");
    let id = adventure_id.to_string();

    let acts: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM act WHERE adventure_id = ?1")
        .bind(&id)
        .fetch_one(&mut *conn)
        .await
        .expect("count acts");
    assert_eq!(acts, seed.acts.len() as u16);

    let active: u16 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM act WHERE adventure_id = ?1 AND status = 'active'",
    )
    .bind(&id)
    .fetch_one(&mut *conn)
    .await
    .expect("count active acts");
    assert_eq!(active, 1, "exactly one act should start active");

    let facts: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM fact WHERE adventure_id = ?1")
        .bind(&id)
        .fetch_one(&mut *conn)
        .await
        .expect("count facts");
    assert_eq!(facts, seed.facts.len() as u16);

    let messages: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM message WHERE adventure_id = ?1")
        .bind(&id)
        .fetch_one(&mut *conn)
        .await
        .expect("count messages");
    assert_eq!(messages, seed.messages.len() as u16);

    let (day, place): (String, String) =
        sqlx::query_as("SELECT day, place FROM state WHERE adventure_id = ?1")
            .bind(&id)
            .fetch_one(&mut *conn)
            .await
            .expect("fetch state");
    assert_eq!(day, seed.state.day);
    assert_eq!(place, seed.state.place);
}

#[tokio::test]
async fn seeded_player_messages_carry_the_player_character_id() {
    let db = fresh_db().await;
    let seed = &SEED_ADVENTURES[0];

    let adventure_id = seed_one(&db.0, seed.title).await.expect("seed one");

    let pool = open_pool(&db.0).await.expect("reopen pool");
    let mut conn = pool.acquire().await.expect("acquire");

    let player_lines: u16 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM message WHERE adventure_id = ?1 AND speaker_name = 'You'",
    )
    .bind(adventure_id.to_string())
    .fetch_one(&mut *conn)
    .await
    .expect("count player lines");
    assert!(player_lines >= 1, "seed should include a player line");

    let unresolved: u16 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM message
         WHERE adventure_id = ?1 AND speaker_name = 'You' AND character_id IS NULL",
    )
    .bind(adventure_id.to_string())
    .fetch_one(&mut *conn)
    .await
    .expect("count unresolved player lines");
    assert_eq!(unresolved, 0, "player lines must carry the player's id");
}
