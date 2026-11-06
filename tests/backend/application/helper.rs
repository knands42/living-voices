use living_voices::ai::integrations::text::chat::Chat;
use living_voices::backend::application::adventure_manager::AdventureManager;
use living_voices::backend::application::character_manager::CharacterManager;
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::domain::memory::{Act, ActStatus};
use living_voices::backend::domain::message::Message;
use living_voices::backend::{open_pool, run_migrations};
use living_voices::config::Config;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use uuid::Uuid;

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
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("living_voices_memory_test_{}.db", Uuid::new_v4()));
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

    let updated_act = adventure_mgr
        .current_snapshot()
        .current_act
        .expect("act should exist");

    adventure_mgr
        .upsert_act(&Act {
            id: updated_act.id,
            adventure_id: adventure.id,
            ordinal: updated_act.ordinal,
            goal: "Reach the frozen tavern".to_string(),
            trigger: None,
            status: ActStatus::Active,
            index_line: updated_act.index_line,
            digest: updated_act.digest,
            running_digest: updated_act.running_digest,
            digest_to_seq: updated_act.digest_to_seq,
            from_seq: updated_act.from_seq,
            to_seq: updated_act.to_seq,
            last_closure_score: updated_act.last_closure_score,
            created_at: updated_act.created_at,
            updated_at: updated_act.updated_at,
        })
        .await
        .expect("update existing act");

    adventure_mgr
        .upsert_act(&Act {
            id: Uuid::new_v4(),
            adventure_id: adventure.id,
            ordinal: 1,
            goal: "Reach the burned tavern".to_string(),
            trigger: None,
            status: ActStatus::Unstarted,
            index_line: Some("Journey towards the legendary burned tavern".to_string()),
            digest: None,
            running_digest: None,
            digest_to_seq: 0,
            from_seq: 0,
            to_seq: 0,
            last_closure_score: 0,
            created_at: updated_act.created_at,
            updated_at: updated_act.updated_at,
        })
        .await
        .expect("update existing act");

    adventure_mgr
        .start_adventure(adventure.id)
        .await
        .expect("start adventure");

    adventure_mgr
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
    // ---- BEGINNING: the door on the tide flat ----
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
        speaker: Some(DEFAULT_CAST_KAEL),
        text: "Maybe it came out somewhere else. Look past the frame — that's not more \
               sand in there. That's a harbour. Lit, crowded, and frozen mid-step.",
    },
    // ---- MIDDLE: the crossing ----
    SeedLine {
        speaker: Some(DEFAULT_CAST_MIRA),
        text: "Every face turned toward us, and not one of them moving. The still ones \
               are the ones already taken. We cross as visitors who leave again.",
    },
    SeedLine {
        speaker: Some("You"),
        text: "The frost takes the boards an arm's length ahead of my boot and stops, \
               like a tide line. Whatever rules this place, it's patient — it lets us \
               walk, so long as we keep walking toward the tavern.",
    },
    SeedLine {
        speaker: Some(DEFAULT_CAST_KAEL),
        text: "The lamplight down the lane just brightened. Not warming — brightening. \
               Like something lighting the way in, or lighting us up for the table.",
    },
    SeedLine {
        speaker: Some(DEFAULT_CAST_MIRA),
        text: "Don't touch the fish strung up on that rack. They're not fish anymore, \
               and their eyes followed me past. Keep each other in sight. Nothing else.",
    },
    SeedLine {
        speaker: Some("You"),
        text: "The whole harbour just drew one breath. The lamps dim, the crowd leans \
               one degree toward the tavern door, and I can hear singing under the \
               frost. It knows we're here, and it knows where we're going.",
    },
    // ---- END: arrival at the frozen tavern ----
    SeedLine {
        speaker: Some(DEFAULT_CAST_KAEL),
        text: "There — end of the lane. A tavern with every window lit and frozen \
               over, frost on the inside of the glass. The sign above the door still \
               swings, though there's no wind anywhere else.",
    },
    SeedLine {
        speaker: Some(DEFAULT_CAST_MIRA),
        text: "The door is not frozen shut. Of everything here, only the door that \
               wants us opens freely. That's an invitation I don't like any better \
               than the last one.",
    },
    SeedLine {
        speaker: Some("You"),
        text: "Then we open it together, or not at all. Warm light under the crack, \
               a chair drawn out like someone's expecting us. We reached the frozen \
               tavern — now we find out what it wanted us here for.",
    },
    SeedLine {
        speaker: Some(DEFAULT_CAST_MIRA),
        text: "Let's rent a room, tomorrow is another we go on a hunt.",
    },
];
