use sqlx::query;
use sqlx::query_scalar;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::{
    CharacterSpec, create_adventure, create_messages, setup_db,
};
use living_voices::ai::integrations::chat::Chat;
use living_voices::ai::integrations::narrator::Narrator;
use living_voices::ai::{ActClose, MemoryUpdate};
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

fn memory_manager(state: SharedState, pool: &SqlitePool) -> MemoryManager<MemoryUpdate, ActClose> {
    let memory_model = Config::new().expect("load config").settings.models.memory;
    let distiller = MemoryUpdate::new(memory_model.clone());
    let closer = ActClose::new(memory_model);
    MemoryManager::new(state, pool.clone(), distiller, closer)
}

fn memory_settings() -> MemorySettings {
    Config::new().expect("load config").settings.memory
}

// ---------------------------------------------------------------- director tuning

/// Always routes to the narrator (ramp hits 1.0 at any message count).
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

/// Never narrates, always honours an explicit target -> the targeted character answers.
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

/// Never narrates, never honours a target -> the model picks the voice (autonomous path).
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

/// Autonomous path with a tiny act budget, pushing the model toward an act close.
fn autonomous_and_transition_act_director() -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance: 0.0,
        narrator_change_if_ooc: 0.0,
        act_max_messages: 2,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: 255,
            max: 255,
        },
    })
}

// ---------------------------------------------------------------- fixtures

async fn setup_adventure(pool: &SqlitePool, characters: &[&str]) -> Adventure {
    // The human player is always a cast member (an `is_player` character), so
    // every adventure carries one. The NPCs keep their leading positions
    // (`characters[0]`, `[1]`, ...), with the player appended last.
    let mut chars: Vec<CharacterSpec> = characters.iter().map(|n| CharacterSpec::npc(n)).collect();
    chars.push(CharacterSpec::player("You"));
    create_adventure(pool, &chars).await
}

fn player_character(adventure: &Adventure) -> &Character {
    adventure
        .characters
        .iter()
        .find(|c| c.is_player)
        .expect("fixture has a player character")
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

async fn seed_active_act(pool: &SqlitePool, adventure_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let mut conn = pool.acquire().await.expect("acquire");
    query(
        "INSERT INTO act (id, adventure_id, ordinal, goal, status)
         VALUES (?1, ?2, 1, 'reach the surface', 'active')",
    )
    .bind(id.to_string())
    .bind(adventure_id.to_string())
    .execute(&mut *conn)
    .await
    .expect("insert act");
    id
}

fn load_adventure(
    pool: &SqlitePool,
    adventure: &Adventure,
    director: Director,
) -> MessageManager<Chat, Narrator, MemoryUpdate, ActClose> {
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

fn player_line(adventure: &Adventure, text: &str) -> Message {
    let player = player_character(adventure);
    Message::new(
        adventure.id,
        Some(player.id),
        None,
        player.name.clone(),
        text,
        false,
        vec![],
    )
}

/// A player line explicitly aimed at one cast member.
fn targeted_line(adventure: &Adventure, text: &str, target: &Character) -> Message {
    let player = player_character(adventure);
    Message::new(
        adventure.id,
        Some(player.id),
        None,
        player.name.clone(),
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

/// Poll briefly for the spawned distillation to stamp the act's watermark.
async fn poll_watermark_set(pool: &SqlitePool, act_id: Uuid) -> bool {
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let mut conn = pool.acquire().await.expect("acquire");
        let watermark: Option<i64> = query_scalar("SELECT digest_to_seq FROM act WHERE id = ?1")
            .bind(act_id.to_string())
            .fetch_one(&mut *conn)
            .await
            .expect("read watermark");
        if watermark.is_some() {
            return true;
        }
    }
    false
}

async fn fact_count(pool: &SqlitePool, act_id: Uuid) -> u16 {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT COUNT(*) FROM fact WHERE act_id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("fact count")
}

/// The `to_seq` of the most recent terminal act, or `None` if none closed.
async fn closed_act_to_seq(pool: &SqlitePool, adventure_id: Uuid) -> Option<i64> {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar(
        "SELECT to_seq FROM act
         WHERE adventure_id = ?1 AND status IN ('resolved', 'failed', 'bypassed')
         ORDER BY ordinal DESC LIMIT 1",
    )
    .bind(adventure_id.to_string())
    .fetch_optional(&mut *conn)
    .await
    .expect("read closed act")
}

async fn message_is_narrator(pool: &SqlitePool, adventure_id: Uuid, seq: i64) -> bool {
    let mut conn = pool.acquire().await.expect("acquire");
    let narrator_id: Option<String> =
        query_scalar("SELECT narrator_id FROM message WHERE adventure_id = ?1 AND seq = ?2")
            .bind(adventure_id.to_string())
            .bind(seq)
            .fetch_one(&mut *conn)
            .await
            .expect("read message");
    narrator_id.is_some()
}

// ---------------------------------------------------------------- 1. full pipeline (distill)

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn full_pipeline_sends_a_turn_and_triggers_background_distillation() {
    // given: an act preloaded to the compression threshold, one more turn away
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira"]).await;
    seed_present_state(&pool, adventure.id, &["Mira"]).await;
    let act_id = seed_active_act(&pool, adventure.id).await;

    let threshold = memory_settings().compress_after_threshold as usize;
    let manager = load_adventure(&pool, &adventure, autonomous_and_transition_act_director());
    create_messages(&pool, adventure.id, &vec![None; threshold]).await;

    // when: the player sends the turn that crosses the threshold
    let incoming = targeted_line(&adventure, "What now?", &adventure.characters[0]);
    let reply = manager.send_message(incoming).await.expect("send succeeds");

    // then: the reply is persisted
    assert!(reply.seq.is_some());
    assert!(!reply.text.trim().is_empty());

    // and: background distillation advances the act watermark
    assert!(
        poll_watermark_set(&pool, act_id).await,
        "background distillation should advance the act watermark"
    );

    // and: it writes at least one fact
    assert!(
        fact_count(&pool, act_id).await >= 1,
        "distillation should write at least one fact"
    );

    // and: if the beat closed the act, its final message is the narrator's
    if let Some(to_seq) = closed_act_to_seq(&pool, adventure.id).await {
        assert!(
            message_is_narrator(&pool, adventure.id, to_seq).await,
            "the completed act should end on the narrator's closing line"
        );
    }
}

// ---------------------------------------------------------------- 2. narrator is picked

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_routes_to_the_narrator() {
    // given: an active act and a director that always narrates
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira"]).await;
    seed_present_state(&pool, adventure.id, &["Mira"]).await;
    seed_active_act(&pool, adventure.id).await;
    let manager = load_adventure(&pool, &adventure, narrator_director());

    // when: the player speaks
    let reply = manager
        .send_message(player_line(&adventure, "Is anyone there?"))
        .await
        .expect("send succeeds");

    // then: the narrator answers (narrator id, no character id)
    assert_eq!(reply.narrator_id, Some(adventure.narrator.id));
    assert!(reply.character_id.is_none());
    assert_eq!(reply.speaker_name, Message::NARRATOR_SPEAKER_NAME);
    assert_eq!(message_count(&pool, adventure.id).await, 2);
}

// ---------------------------------------------------------------- 3. a random character is picked

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_routes_to_a_random_cast_member() {
    // given: two present characters and a director that defers the voice to the model
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira", "Kael"]).await;
    seed_present_state(&pool, adventure.id, &["Mira", "Kael"]).await;
    seed_active_act(&pool, adventure.id).await;
    let manager = load_adventure(&pool, &adventure, autonomous_director());

    // when: the player invites anyone to speak
    let reply = manager
        .send_message(player_line(&adventure, "Someone say something."))
        .await
        .expect("send succeeds");

    // then: a present cast member answers, not the narrator
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
    // given: two present characters and a director that honours an explicit target
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira", "Kael"]).await;
    seed_present_state(&pool, adventure.id, &["Mira", "Kael"]).await;
    seed_active_act(&pool, adventure.id).await;
    let manager = load_adventure(&pool, &adventure, specific_character_director());

    // when: the player aims the line at Kael
    let kael = adventure.characters[1].clone();
    let reply = manager
        .send_message(targeted_line(&adventure, "Kael, what do you see?", &kael))
        .await
        .expect("send succeeds");

    // then: Kael answers, not the narrator
    assert_eq!(reply.character_id, Some(kael.id));
    assert_eq!(reply.speaker_name, "Kael");
    assert!(reply.narrator_id.is_none());
    assert_eq!(message_count(&pool, adventure.id).await, 2);
}
