use living_voices::backend::{open_pool, run_migrations};
use sqlx::SqlitePool;
use std::path::PathBuf;
use uuid::Uuid;

use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::Character;
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::characters_persistence::insert_character;
use living_voices::backend::persistence::message_persistence::insert_message;
use living_voices::backend::persistence::narrator_persistence::insert_narrator;

pub struct TempDb(pub PathBuf);
impl Drop for TempDb {
    fn drop(&mut self) {
        let base = self.0.to_string_lossy().into_owned();
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{base}{suffix}"));
        }
    }
}

pub async fn setup_db() -> (SqlitePool, TempDb) {
    let path =
        std::env::temp_dir().join(format!("living_voices_memory_test_{}.db", Uuid::new_v4()));
    let db = TempDb(path);
    let db_str = db.0.to_string_lossy().into_owned();

    // Given migrations applied on a fresh file
    let migrating = open_pool(&db_str).await.expect("open pool");
    run_migrations(migrating).await.expect("run migrations");
    // `run_migrations` consumes its pool, so open a fresh one for the test
    let pool = open_pool(&db_str).await.expect("reopen pool");

    (pool, db)
}

/// One character to create: its name and whether it is the player-controlled one.
pub struct CharacterSpec {
    pub name: String,
    pub is_player: bool,
}

impl CharacterSpec {
    pub fn npc(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_player: false,
        }
    }

    pub fn player(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_player: true,
        }
    }
}

impl From<&str> for CharacterSpec {
    fn from(name: &str) -> Self {
        CharacterSpec::npc(name)
    }
}

/// Build (but do not persist) a cast from specs, all tied to `adventure_id`.
pub fn build_characters(adventure_id: Uuid, specs: &[CharacterSpec]) -> Vec<Character> {
    specs
        .iter()
        .map(|spec| {
            let character = Character::new(spec.name.clone(), Some(adventure_id), "A diver".into());
            if spec.is_player {
                character.set_as_player()
            } else {
                character
            }
        })
        .collect()
}

/// Persist a cast under `adventure_id` (the adventure row must already exist).
pub async fn create_characters(
    pool: &SqlitePool,
    adventure_id: Uuid,
    specs: &[CharacterSpec],
) -> Vec<Character> {
    let characters = build_characters(adventure_id, specs);
    let mut conn = pool.acquire().await.expect("acquire");
    for character in &characters {
        insert_character(&mut conn, Some(adventure_id), character)
            .await
            .expect("insert character");
    }
    characters
}

/// Persist a full adventure: the adventure row, its narrator, and the given cast.
/// Returns the `Adventure` carrying the persisted characters.
pub async fn create_adventure(pool: &SqlitePool, specs: &[CharacterSpec]) -> Adventure {
    let mut adventure = Adventure::new(
        "Test adventure".into(),
        "A drowned city".into(),
        "Guide".into(),
        "tense",
    );
    adventure.characters = build_characters(adventure.id, specs);

    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, &adventure)
        .await
        .expect("insert adventure");
    insert_narrator(&mut conn, adventure.id, &adventure.narrator)
        .await
        .expect("insert narrator");
    for character in &adventure.characters {
        insert_character(&mut conn, Some(adventure.id), character)
            .await
            .expect("insert character");
    }
    adventure
}

pub async fn create_messages(
    pool: &SqlitePool,
    adventure_id: Uuid,
    authors: &[Option<Uuid>],
) -> Vec<u16> {
    let mut conn = pool.acquire().await.expect("acquire");
    let mut seqs = Vec::with_capacity(authors.len());
    for (i, author) in authors.iter().enumerate() {
        let message = Message::new(
            adventure_id,
            *author,
            None,
            format!("Speaker {i}"),
            format!("line {i}"),
            false,
            vec![],
        );
        let stored = insert_message(&mut conn, &message)
            .await
            .expect("insert message");
        seqs.push(stored.seq.expect("assigned seq"));
    }
    seqs
}
