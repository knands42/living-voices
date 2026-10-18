use sqlx::query_scalar;
use sqlx::sqlite::SqlitePool;

use crate::backend::application::helper::setup_db;
use living_voices::ai::MemoryUpdate;
use living_voices::ai::jobs::chat::Chat;
use living_voices::ai::jobs::narrator::Narrator;
use living_voices::backend::application::director::Director;
use living_voices::backend::application::memory_manager::MemoryManager;
use living_voices::backend::application::message_manager::{MessageManager, MessageManagerError};
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::Character;
use living_voices::backend::domain::message::{Message, MessageKind, MessagePart, MessageParts};
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::characters_persistence::insert_character;
use living_voices::backend::persistence::narrator_persistence::insert_narrator;
use living_voices::config::{Config, TextModel};

// ---------------------------------------------------------------- helpers

fn chat_model() -> TextModel {
    let config = Config::new().expect("load config");
    config.settings.models.chat
}

/// Build the chat handler the app wires in, from the configured chat model.
fn chat_handler() -> Chat {
    Chat::new(chat_model())
}

/// The narrator shares the chat text endpoint, as the app wires it.
fn narrator_handler() -> Narrator {
    Narrator::new(chat_model())
}

fn memory_manager(state: SharedState, pool: &SqlitePool) -> MemoryManager<MemoryUpdate> {
    let config = Config::new().expect("load config");
    let distiller = MemoryUpdate::new(config.settings.models.memory);
    MemoryManager::new(state, pool.clone(), distiller)
}

fn memory_settings() -> living_voices::config::MemorySettings {
    Config::new().expect("load config").settings.memory
}

async fn setup_adventure(pool: &SqlitePool, characters: &[&str]) -> Adventure {
    let mut adventure = Adventure::new(
        "Test adventure".into(),
        "Description".into(),
        "Guide".into(),
        "",
    );
    for name in characters {
        let character = Character::new(
            (*name).to_string(),
            Some(adventure.id),
            "A diver".to_string(),
        );
        adventure.characters.push(character);
    }
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

fn load_adventure(
    pool: &SqlitePool,
    adventure: &Adventure,
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
        Director::new(1.0, Default::default()),
        memory,
        memory_settings(),
    )
}

fn player_line(adventure_id: uuid::Uuid, speaker: &str, text: &str) -> Message {
    Message::new(
        adventure_id,
        None,
        None,
        speaker.to_string(),
        MessageParts::one(MessagePart::new(MessageKind::Say, text)),
    )
}

// ---------------------------------------------------------------- tests

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_persists_incoming_and_reply_and_returns_the_stored_reply() {
    // Given an adventure with one character loaded in the state
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira"]).await;
    let manager = load_adventure(&pool, &adventure);

    // When sending one player line through the real chat model
    let incoming = player_line(adventure.id, "Player", "What do you see around us?");
    let reply = manager.send_message(incoming).await.expect("send succeeds");

    // Then a reply is persisted with an assigned seq and non-empty text
    assert!(reply.seq.is_some());
    assert!(!reply.parts.primary().text.trim().is_empty());

    // And both the incoming line and the reply are stored
    let mut conn = pool.acquire().await.expect("acquire");
    let count: i64 = query_scalar("SELECT COUNT(*) FROM message WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count messages");
    assert_eq!(count, 2);
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_attributes_the_reply_to_a_cast_member_or_the_narrator() {
    // Given an adventure whose cast includes "Mira"
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira"]).await;
    let manager = load_adventure(&pool, &adventure);

    // When sending a player line
    let incoming = player_line(adventure.id, "Player", "Is anyone there?");
    let reply = manager.send_message(incoming).await.expect("send succeeds");

    // Then the model picked either the character or the narrator, never both, never neither
    match (reply.character_id, reply.narrator_id) {
        (Some(character_id), None) => {
            assert_eq!(character_id, adventure.characters[0].id);
            assert_eq!(reply.speaker_name, "Mira");
            assert_eq!(reply.parts.primary().kind, MessageKind::Say);
        }
        (None, Some(narrator_id)) => {
            assert_eq!(narrator_id, adventure.narrator.id);
            assert_eq!(reply.speaker_name, Message::NARRATOR_SPEAKER_NAME);
            assert_eq!(reply.parts.primary().kind, MessageKind::Narration);
        }
        other => panic!("reply must be a character XOR the narrator, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn send_message_carries_prior_turns_into_the_next_turn() {
    // Given one turn already exchanged
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &["Mira"]).await;
    let manager = load_adventure(&pool, &adventure);

    manager
        .send_message(player_line(adventure.id, "Player", "My name is Kael."))
        .await
        .expect("first send");

    // When sending a second line, the second turn succeeds with the earlier turn in the log
    let reply = manager
        .send_message(player_line(
            adventure.id,
            "Player",
            "Do you remember my name?",
        ))
        .await
        .expect("second send");

    // Then four messages are stored (two player lines + two replies)
    assert!(reply.seq.is_some());
    let mut conn = pool.acquire().await.expect("acquire");
    let count: i64 = query_scalar("SELECT COUNT(*) FROM message WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count messages");
    assert_eq!(count, 4);
}

#[tokio::test]
async fn send_message_errors_without_a_current_adventure() {
    // Given a manager with no adventure loaded (no model call is reached)
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let memory = memory_manager(state.clone(), &pool);
    let manager = MessageManager::new(
        state,
        pool.clone(),
        chat_handler(),
        narrator_handler(),
        Director::new(1.0, Default::default()),
        memory,
        memory_settings(),
    );

    // When sending a message
    let incoming = player_line(uuid::Uuid::new_v4(), "Player", "Hello");
    let result = manager.send_message(incoming).await;

    // Then it fails with NoAdventure and nothing is persisted
    assert!(matches!(result, Err(MessageManagerError::NoAdventure)));
    let mut conn = pool.acquire().await.expect("acquire");
    let count: i64 = query_scalar("SELECT COUNT(*) FROM message")
        .fetch_one(&mut *conn)
        .await
        .expect("count messages");
    assert_eq!(count, 0);
}
