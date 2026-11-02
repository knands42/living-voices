use sqlx::query_scalar;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::{seed_message, seed_messages, setup_db};
use living_voices::ai::integrations::text::chat::Chat;
use living_voices::ai::integrations::text::narrator::Narrator;
use living_voices::ai::{ActClose, MemoryUpdate};
use living_voices::backend::application::adventure_manager::AdventureManager;

use living_voices::backend::application::character_manager::CharacterManager;
use living_voices::backend::application::director::Director;
use living_voices::backend::application::memory_manager::MemoryManager;
use living_voices::backend::application::message_manager::MessageManager;
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::act_persistence::get_active_act;
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

fn memory_high_distiller_trigger() -> MemorySettings {
    Config::new().expect("load config").settings.memory
}

fn memory_low_distiller_trigger() -> MemorySettings {
    MemorySettings {
        compress_after_threshold: 10,
        compress_last_messages: 5,
    }
}

// ---------------------------------------------------------------- director tuning

fn director_pick_narrator() -> Director {
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

fn director_pick_character() -> Director {
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

fn director_pick_autonomous() -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance: 0.0,
        narrator_change_if_ooc: 0.0,
        act_max_messages: 10_000,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: 0,
            max: 0,
        },
    })
}

fn director_transition_act() -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance: 0.0,
        narrator_change_if_ooc: 0.0,
        act_max_messages: 5,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: 255,
            max: 255,
        },
    })
}

// ---------------------------------------------------------------- fixtures

async fn setup_new_adventure(
    pool: &SqlitePool,
    characters: &[&str],
) -> (SharedState, AdventureManager) {
    let state = SharedState::new();
    let adventure_mgr = AdventureManager::new(state.clone(), pool.clone());
    let adventure = adventure_mgr
        .create_adventure(
            "Test adventure".into(),
            "A drowned city".into(),
            "tense".into(),
        )
        .await
        .expect("create adventure");

    // Seed any additional NPCs beyond the default player character
    let character_mgr = CharacterManager::new(state.clone(), pool.clone(), chat_handler());
    if !characters.is_empty() {
        let characters_mapped = characters.iter().map(|char_name| {
            Character::new(
                Uuid::new_v4(),
                (*char_name).into(),
                adventure.id,
                format!("Character {} headline", *char_name).as_str(),
                format!("Character {} backstory", *char_name).as_str(),
                CharacterStatus::Present,
                false,
            )
        });

        for character in characters_mapped {
            character_mgr
                .create_character(character.clone())
                .await
                .expect("create character");
        }
    }

    adventure_mgr
        .start_adventure(adventure.clone())
        .await
        .expect("start adventure");

    (state, adventure_mgr)
}

fn player_character(adventure: &Adventure) -> &Character {
    adventure
        .characters
        .iter()
        .find(|c| c.is_player)
        .expect("fixture has a player character")
}

fn setup_message_manager(
    state: &SharedState,
    pool: &SqlitePool,
    director: &Director,
    memory_config: &MemorySettings,
) -> MessageManager<Chat, Narrator, MemoryUpdate, ActClose> {
    let memory = memory_manager(state.clone(), pool);
    MessageManager::new(
        state.clone(),
        pool.clone(),
        chat_handler(),
        narrator_handler(),
        director.clone(),
        memory,
        memory_config.clone(),
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
    )
}

async fn check_digest_checkpoint(pool: &SqlitePool, act_id: Uuid) -> bool {
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
async fn full_pipeline_sends_a_turn_and_triggers_an_act_closure() {}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn full_pipeline_sends_a_turn_and_triggers_background_distillation() {
    // given: an act preloaded to the compression threshold, one more turn away
    let (pool, _db) = setup_db().await;
    let (state, adventure_manager) = setup_new_adventure(&pool, &["Mira"]).await;
    let memory_config = memory_low_distiller_trigger();
    let manager = setup_message_manager(&state, &pool, &director_pick_autonomous(), &memory_config);

    let adventure = adventure_manager.current_adventure().unwrap();
    let threshold = memory_config.compress_after_threshold as usize;
    let characters = adventure.characters.clone();
    let mira_character = characters.iter().find(|c| c.name == "Mira").unwrap();
    let player_character = characters.iter().find(|c| c.is_player).unwrap();

    seed_messages(
        &pool,
        adventure.id,
        &[mira_character.clone(), player_character.clone()],
        threshold,
    )
    .await;

    // Get the active act
    let mut conn = pool.acquire().await.expect("acquire");
    let act_id = get_active_act(&mut conn, adventure.id)
        .await
        .expect("get active act")
        .expect("has active act")
        .id;
    drop(conn);

    // when: the player sends the turn that crosses the threshold
    let incoming = player_line(&adventure, "What now?");
    let reply = manager.send_message(incoming).await.expect("send succeeds");

    // then: the reply is persisted
    assert!(reply.seq.is_some());
    assert!(!reply.text.trim().is_empty());

    // and: background distillation advances the act checkpoint
    assert!(
        check_digest_checkpoint(&pool, act_id).await,
        "background distillation should advance the act checkpoint"
    );

    // and: it writes at least one fact
    assert!(
        fact_count(&pool, act_id).await >= 1,
        "distillation should write at least one fact"
    );
}

// ---------------------------------------------------------------- 2. narrator is picked

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_routes_to_the_narrator() {
    // given: an active act and a director that always narrates
    let (pool, _db) = setup_db().await;
    let (state, adventure_manager) = setup_new_adventure(&pool, &["Mira"]).await;
    let memory_config = memory_high_distiller_trigger();
    let manager = setup_message_manager(&state, &pool, &director_pick_narrator(), &memory_config);
    let adventure = adventure_manager.current_adventure().unwrap();

    // when: the player speaks
    let reply = manager
        .send_message(player_line(&adventure, "Is anyone there?"))
        .await
        .expect("send succeeds");

    // then: the narrator answers (narrator id, no character id)
    assert_eq!(reply.narrator_id, Some(adventure.narrator.id));
    assert!(reply.character_id.is_none());
    assert_eq!(reply.speaker_name, Message::NARRATOR_SPEAKER_NAME);
}

// ---------------------------------------------------------------- 3. a random character is picked

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_routes_to_a_random_cast_member() {
    // given: two present characters and a director that defers the voice to the model
    let (pool, _db) = setup_db().await;
    let (state, adventure_manager) = setup_new_adventure(&pool, &["Mira"]).await;
    let adventure = adventure_manager.current_adventure().unwrap();
    let memory_config = memory_high_distiller_trigger();
    let manager = setup_message_manager(&state, &pool, &director_pick_narrator(), &memory_config);

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
}

// ---------------------------------------------------------------- 4. a specific character is picked

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_routes_to_a_specific_targeted_character() {
    // given: two present characters and a director that honours an explicit target
    let (pool, _db) = setup_db().await;
    let (state, adventure_manager) = setup_new_adventure(&pool, &["Mira"]).await;
    let adventure = adventure_manager.current_adventure().unwrap();
    let memory_config = memory_high_distiller_trigger();
    let manager = setup_message_manager(&state, &pool, &director_pick_character(), &memory_config);

    // when: the player aims the line at Kael
    let kael = adventure
        .characters
        .iter()
        .find(|c| c.name == "Kael")
        .expect("Kael is in the cast")
        .clone();
    let reply = manager
        .send_message(player_line(&adventure, "Kael, what do you see?"))
        .await
        .expect("send succeeds");

    // then: Kael answers, not the narrator
    assert_eq!(reply.character_id, Some(kael.id));
    assert_eq!(reply.speaker_name, "Kael");
    assert!(reply.narrator_id.is_none());
}
