use living_voices::backend::{open_pool, run_migrations};
use sqlx::SqlitePool;
use std::path::PathBuf;
use uuid::Uuid;

use living_voices::backend::application::character_manager::CharacterManager;
use living_voices::backend::application::port::chat_port::{
    ChatResponseError, PromptHandler, PromptTurn,
};
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::domain::message::Message;

pub struct StubPromptHandler;

impl PromptHandler for StubPromptHandler {
    async fn handle_prompt(&self, _prompt: &[PromptTurn]) -> Result<String, ChatResponseError> {
        Ok(String::new())
    }
}

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
    run_migrations(&migrating).await.expect("run migrations");
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
            Character::new(
                Uuid::new_v4(),
                spec.name.clone(),
                adventure_id,
                "A diver",
                "",
                CharacterStatus::Present,
                spec.is_player,
            )
        })
        .collect()
}

/// Persist a cast under `adventure_id` using CharacterManager (the adventure row must already exist).
pub async fn create_characters(
    pool: &SqlitePool,
    adventure_id: Uuid,
    specs: &[CharacterSpec],
) -> Vec<Character> {
    let characters = build_characters(adventure_id, specs);
    let state = SharedState::new();
    let character_mgr = CharacterManager::new(state, pool.clone(), StubPromptHandler);
    for character in &characters {
        character_mgr
            .create_character(character.clone())
            .await
            .expect("create character");
    }
    characters
}

pub async fn seed_messages(
    pool: &SqlitePool,
    adventure_id: Uuid,
    characters: &[Character],
    count: usize,
) {
    let cap = characters.len();
    for _ in 0..count {
        let roll = rand::random::<u8>() as usize;
        seed_message(
            &pool,
            adventure_id,
            characters[roll % cap].id,
            &characters[roll % cap].name,
        )
        .await;
    }
}

/// Seed raw messages via persistence; MessageManager::send_message always calls the model.
pub async fn seed_message(
    pool: &SqlitePool,
    adventure_id: Uuid,
    character_id: Uuid,
    character_name: &str,
) -> Message {
    use living_voices::backend::domain::message::Message;
    use living_voices::backend::persistence::message_persistence::insert_message;

    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure_id,
        Some(character_id),
        None,
        character_name.into(),
        "Lorem Ipsum".to_string(),
        false,
    );
    let stored = insert_message(&mut conn, &message)
        .await
        .expect("insert message");

    stored
}
