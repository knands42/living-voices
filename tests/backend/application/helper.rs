use living_voices::ai::integrations::text::chat::Chat;
use living_voices::backend::application::adventure_manager::AdventureManager;
use living_voices::backend::application::character_manager::CharacterManager;
use living_voices::backend::application::port::chat_port::{
    ChatResponseError, PromptHandler, PromptTurn,
};
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::domain::message::Message;
use living_voices::backend::{open_pool, run_migrations};
use living_voices::config::Config;
use sqlx::SqlitePool;
use std::path::PathBuf;
use uuid::Uuid;

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

pub async fn start_adventure_with_default_cast(
    pool: &SqlitePool,
    state: &SharedState,
) -> AdventureManager {
    let adventure_mgr = AdventureManager::new(state.clone(), pool.clone());
    let adventure = adventure_mgr
        .create_adventure(
            "Test adventure".into(),
            "A drowned city".into(),
            "tense".into(),
        )
        .await
        .expect("create adventure");

    let chat_handler = Chat::new(Config::new().expect("load config").settings.models.chat);
    let character_mgr = CharacterManager::new(state.clone(), pool.clone(), chat_handler);
    let npcs = vec![DEFAULT_CAST_MIRA, DEFAULT_CAST_KAEL];
    for npc in npcs {
        let character = Character::new(
            Uuid::new_v4(),
            (*npc).into(),
            adventure.id,
            format!("Character {npc} headline").as_str(),
            format!("Character {npc} backstory").as_str(),
            CharacterStatus::Present,
            false,
        );
        character_mgr
            .create_character(character)
            .await
            .expect("create character");
    }

    adventure_mgr
        .start_adventure(adventure.id)
        .await
        .expect("start adventure");

    adventure_mgr
}

pub async fn create_characters(
    pool: &SqlitePool,
    adventure_id: Uuid,
    specs: &[CharacterSpec],
) -> Vec<Character> {
    let characters: Vec<Character> = specs
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
        .collect();

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

pub struct SeedLine {
    pub speaker: Option<&'static str>,
    pub text: &'static str,
}

pub async fn seed_conversation(
    pool: &SqlitePool,
    adventure: &Adventure,
    lines: &[SeedLine],
    count: usize,
) -> Vec<u16> {
    use living_voices::backend::persistence::message_persistence::insert_message;

    let mut conn = pool.acquire().await.expect("acquire");
    let mut seqs = Vec::with_capacity(count.min(lines.len()));
    for line in lines.iter().take(count) {
        let speaker = line.speaker.unwrap_or(Message::NARRATOR_SPEAKER_NAME);
        let message = adventure.message_from_reply(speaker, line.text);
        let stored = insert_message(&mut conn, &message)
            .await
            .expect("insert message");
        seqs.push(stored.seq.expect("seeded message has a seq"));
    }
    seqs
}

pub const DEFAULT_CAST_MIRA: &'static str = "Mira";
pub const DEFAULT_CAST_KAEL: &'static str = "Kael";
pub const CONVERSATION: &[SeedLine] = &[
    SeedLine {
        speaker: None,
        text: "The tide has gone out further than the charts allow. Where the harbour \
               should be there is only wet sand, and a door standing open in the middle \
               of it, leading nowhere.",
    },
    SeedLine {
        speaker: Some("You"),
        text: "I don't trust a door with no walls. Mira, have you ever seen the water \
               pull back this far?",
    },
    SeedLine {
        speaker: Some(DEFAULT_CAST_MIRA),
        text: "Once. The night my brother didn't come home. The sea gives you a path \
               like this when it wants something walked toward it.",
    },
    SeedLine {
        speaker: Some("You"),
        text: "Then we walk it carefully. A gull just flew through that frame and never \
               came out the far side. Tell me you saw that too.",
    },
    SeedLine {
        speaker: Some(DEFAULT_CAST_MIRA),
        text: "I saw it. And the sand around the threshold is dry, in the middle of a \
               tide flat. That's not the sea's doing. That's an invitation.",
    },
    SeedLine {
        speaker: Some("You"),
        text: "An invitation I'd rather decline. But my brother's name was the last \
               thing the water took from you — so I won't make you turn back.",
    },
    SeedLine {
        speaker: Some(DEFAULT_CAST_KAEL),
        text: "Then stay close. Through that frame I can see the harbour, lit and \
               crowded and ordinary — except every face is turned to us, and not one of \
               them is moving.",
    },
    SeedLine {
        speaker: Some("You"),
        text: "The still ones are the ones already taken. If we cross, we cross as \
               visitors who leave again, not as names added to their list.",
    },
    SeedLine {
        speaker: Some(DEFAULT_CAST_MIRA),
        text: "Agreed. We keep each other in sight, we touch nothing, and the moment the \
               lamplight dims we come straight back through the door.",
    },
    SeedLine {
        speaker: Some("You"),
        text: "The lamplight is dimming now, like the whole frozen harbour drew one \
               breath. Whatever waits past the threshold, it knows we're here.",
    },
];
