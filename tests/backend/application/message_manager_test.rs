use sqlx::query;
use sqlx::query_scalar;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::{
    CharacterSpec, create_adventure, create_messages, setup_db,
};
use living_voices::ai::MemoryUpdate;
use living_voices::ai::integrations::chat::Chat;
use living_voices::ai::integrations::narrator::Narrator;
use living_voices::backend::application::director::Director;
use living_voices::backend::application::memory_manager::MemoryManager;
use living_voices::backend::application::message_manager::MessageManager;
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::Character;
use living_voices::backend::domain::message::Message;
use living_voices::config::{
    Config, DirectorSettings, MemorySettings, MessageThresholdRange, TextModel,
};

// ---------------------------------------------------------------- handler wiring (real models)

fn chat_model() -> TextModel {
    Config::new().expect("load config").settings.models.chat
}

fn chat_handler() -> Chat {
    Chat::new(chat_model())
}

fn narrator_handler() -> Narrator {
    Narrator::new(chat_model())
}

fn memory_manager(state: SharedState, pool: &SqlitePool) -> MemoryManager<MemoryUpdate> {
    let distiller = MemoryUpdate::new(Config::new().expect("load config").settings.models.memory);
    MemoryManager::new(state, pool.clone(), distiller)
}

fn memory_settings() -> MemorySettings {
    Config::new().expect("load config").settings.memory
}

// ---------------------------------------------------------------- director tuning

/// A director that always routes to the NARRATOR: the ramp reaches 1.0 at any
/// message count (min == max == 0, so `messages_since_narration >= max`).
fn narrator_director() -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance: 1.0,
        narrator_change_if_ooc: 1.0,
        act_max_messages: 10_000,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: 0,
            max: 0,
        },
    })
}

/// A director that never narrates and always honours an explicit target, so a
/// targeted message routes to that specific character.
fn specific_character_director() -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance: 1.0,
        narrator_change_if_ooc: 0.0,
        act_max_messages: 10_000,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: 255,
            max: 255,
        },
    })
}

/// A director that never narrates and never honours a target, so the choice of
/// voice is deferred to the model (the autonomous path picks any cast member).
fn autonomous_director() -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance: 0.0,
        narrator_change_if_ooc: 0.0,
        act_max_messages: 10_000,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: 255,
            max: 255,
        },
    })
}

// ---------------------------------------------------------------- fixtures

async fn setup_adventure(pool: &SqlitePool, characters: &[&str]) -> Adventure {
    let specs: Vec<CharacterSpec> = characters.iter().map(|n| CharacterSpec::npc(n)).collect();
    create_adventure(pool, &specs).await
}

/// Mark which cast members are present, so `get_active_candidates` can route to
/// a character branch (an empty/absent state leaves the narrator the only voice).
async fn seed_present_state(pool: &SqlitePool, adventure_id: Uuid, present: &[&str]) {
    let present_json = serde_json::to_string(present).expect("json");
    let mut conn = pool.acquire().await.expect("acquire");
    query(
        "INSERT INTO state (id, adventure_id, characters_present)
         VALUES (?1, ?2, ?3)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(adventure_id.to_string())
    .bind(present_json)
    .execute(&mut *conn)
    .await
    .expect("insert state");
}

async fn seed_active_act(pool: &SqlitePool, adventure_id: Uuid) {
    let mut conn = pool.acquire().await.expect("acquire");
    query(
        "INSERT INTO act (id, adventure_id, ordinal, goal, status)
         VALUES (?1, ?2, 1, 'reach the surface', 'active')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(adventure_id.to_string())
    .execute(&mut *conn)
    .await
    .expect("insert act");
}

fn load_adventure(
    pool: &SqlitePool,
    adventure: &Adventure,
    director: Director,
) -> MessageManager<Chat, Narrator, MemoryUpdate> {
    let state = SharedState::new();
    state
        .write()
        .expect("lock")
        .set_adventure(adventure.clone());
    let memory = memory_manager(state.clone(), pool);
    MessageManager::new(
        state,
        pool.clone(),
        chat_handler(),
        narrator_handler(),
        director,
        memory,
        memory_settings(),
    )
}

fn player_line(adventure_id: Uuid, text: &str) -> Message {
    Message::new(
        adventure_id,
        None,
        None,
        "Player".into(),
        text,
        false,
        vec![],
    )
}

/// A player line explicitly aimed at one cast member.
fn targeted_line(adventure_id: Uuid, text: &str, target: &Character) -> Message {
    Message::new(
        adventure_id,
        None,
        None,
        "Player".into(),
        text,
        false,
        vec![target.clone()],
    )
}

async fn message_count(pool: &SqlitePool, adventure_id: Uuid) -> u16 {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT COUNT(*) FROM message WHERE adventure_id = ?1")
        .bind(adventure_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count")
}

// ---------------------------------------------------------------- 1. full pipeline (distill)

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn full_pipeline_sends_a_turn_and_triggers_background_distillation() {
    // Given an adventure already carrying enough messages that one more crosses the
    // compression threshold, plus an active act for the distiller to fold into.
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira"]).await;
    seed_present_state(&pool, adventure.id, &["Mira"]).await;
    let act_id = {
        // Capture the act id for the post-distill assertions.
        let id = Uuid::new_v4();
        let mut conn = pool.acquire().await.expect("acquire");
        query(
            "INSERT INTO act (id, adventure_id, ordinal, goal, status)
             VALUES (?1, ?2, 1, 'reach the surface', 'active')",
        )
        .bind(id.to_string())
        .bind(adventure.id.to_string())
        .execute(&mut *conn)
        .await
        .expect("insert act");
        id
    };

    let threshold = memory_settings().compress_after_threshold as usize;
    let manager = load_adventure(&pool, &adventure, specific_character_director());

    // Pre-load the log so the NEXT send crosses the threshold inside maybe_compress_memory.
    create_messages(&pool, adventure.id, &vec![None; threshold]).await;

    // When sending one more player turn (targeted so a character answers deterministically)
    let incoming = targeted_line(adventure.id, "What now?", &adventure.characters[0]);
    let reply = manager.send_message(incoming).await.expect("send succeeds");

    // Then the reply is persisted...
    assert!(reply.seq.is_some());
    assert!(!reply.text.trim().is_empty());

    // and writes a running digest. maybe_compress_memory spawns it, so poll briefly.
    let mut digested = false;
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let mut conn = pool.acquire().await.expect("acquire");
        let watermark: Option<i64> = query_scalar("SELECT digest_to_seq FROM act WHERE id = ?1")
            .bind(act_id.to_string())
            .fetch_one(&mut *conn)
            .await
            .expect("read watermark");
        if watermark.is_some() {
            digested = true;
            break;
        }
    }
    assert!(
        digested,
        "background distillation should advance the act watermark"
    );

    let mut conn = pool.acquire().await.expect("acquire");
    let fact_count: u16 = query_scalar("SELECT COUNT(*) FROM fact WHERE act_id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("fact count");
    assert!(
        fact_count >= 1,
        "distillation should write at least one fact"
    );
}

// ---------------------------------------------------------------- 2. narrator is picked

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_routes_to_the_narrator() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira"]).await;
    seed_present_state(&pool, adventure.id, &["Mira"]).await;
    seed_active_act(&pool, adventure.id).await;
    let manager = load_adventure(&pool, &adventure, narrator_director());

    let reply = manager
        .send_message(player_line(adventure.id, "Is anyone there?"))
        .await
        .expect("send succeeds");

    // The narrator answered: narrator id set, no character id, narrator speaker name.
    assert_eq!(reply.narrator_id, Some(adventure.narrator.id));
    assert!(reply.character_id.is_none());
    assert_eq!(reply.speaker_name, Message::NARRATOR_SPEAKER_NAME);
    assert_eq!(message_count(&pool, adventure.id).await, 2);
}

// ---------------------------------------------------------------- 3. a random character is picked

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_routes_to_a_random_cast_member() {
    // Two present characters, narrator suppressed, no explicit target: the model
    // picks a voice on the autonomous path.
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira", "Kael"]).await;
    seed_present_state(&pool, adventure.id, &["Mira", "Kael"]).await;
    seed_active_act(&pool, adventure.id).await;
    let manager = load_adventure(&pool, &adventure, autonomous_director());

    let reply = manager
        .send_message(player_line(adventure.id, "Someone say something."))
        .await
        .expect("send succeeds");

    // A cast member answered (not the narrator); the speaker is one of the present cast.
    assert!(
        reply.narrator_id.is_none(),
        "a character, not the narrator, should answer"
    );
    let present = ["Mira", "Kael"];
    assert!(
        present.contains(&reply.speaker_name.as_str()),
        "reply speaker `{}` should be one of the present cast",
        reply.speaker_name
    );
    assert_eq!(message_count(&pool, adventure.id).await, 2);
}

// ---------------------------------------------------------------- 4. a specific character is picked

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_routes_to_a_specific_targeted_character() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira", "Kael"]).await;
    seed_present_state(&pool, adventure.id, &["Mira", "Kael"]).await;
    seed_active_act(&pool, adventure.id).await;
    let manager = load_adventure(&pool, &adventure, specific_character_director());

    // The player aims the line at Kael specifically.
    let kael = adventure.characters[1].clone();
    let reply = manager
        .send_message(targeted_line(adventure.id, "Kael, what do you see?", &kael))
        .await
        .expect("send succeeds");

    // Kael answered: his character id and name, no narrator.
    assert_eq!(reply.character_id, Some(kael.id));
    assert_eq!(reply.speaker_name, "Kael");
    assert!(reply.narrator_id.is_none());
    assert_eq!(message_count(&pool, adventure.id).await, 2);
}
