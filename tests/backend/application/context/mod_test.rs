use sqlx::query;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::{CharacterSpec, create_adventure, setup_db};
use living_voices::backend::application::context::ContextBuilder;
use living_voices::backend::application::director::Director;
use living_voices::backend::application::port::chat_port::Speaker;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::Character;
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::message_persistence::insert_message;
use living_voices::config::DirectorSettings;

// region DB fixtures

async fn setup_adventure(pool: &SqlitePool, characters: &[(&str, bool)]) -> Adventure {
    let specs: Vec<CharacterSpec> = characters
        .iter()
        .map(|(name, is_player)| {
            if *is_player {
                CharacterSpec::player(name)
            } else {
                CharacterSpec::npc(name)
            }
        })
        .collect();
    create_adventure(pool, &specs).await
}

async fn seed_active_act(pool: &SqlitePool, adventure_id: Uuid, running: Option<&str>) -> Uuid {
    let act_id = Uuid::new_v4();
    let mut conn = pool.acquire().await.expect("acquire");
    query(
        "INSERT INTO act (id, adventure_id, ordinal, goal, status, running_digest)
         VALUES (?1, ?2, 1, 'reach the surface', 'active', ?3)",
    )
    .bind(act_id.to_string())
    .bind(adventure_id.to_string())
    .bind(running)
    .execute(&mut *conn)
    .await
    .expect("insert act");
    act_id
}

/// Insert a player-authored line (character_id of a cast member flagged is_player).
async fn seed_player_message(
    pool: &SqlitePool,
    adventure: &Adventure,
    author: &Character,
    text: &str,
) {
    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure.id,
        Some(author.id),
        None,
        author.name.clone(),
        text,
        false,
        vec![],
    );
    insert_message(&mut conn, &message).await.expect("insert");
}

async fn seed_cast_message(
    pool: &SqlitePool,
    adventure: &Adventure,
    author: &Character,
    text: &str,
) {
    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure.id,
        Some(author.id),
        None,
        author.name.clone(),
        text,
        false,
        vec![],
    );
    insert_message(&mut conn, &message).await.expect("insert");
}

fn low_closure_director() -> Director {
    // A huge act cap so the closure gate never fires in these fixtures.
    Director::new(DirectorSettings {
        act_max_messages: 10_000,
        ..Default::default()
    })
}

// endregion

// region Autonomous prompt

#[tokio::test]
async fn autonomous_prompt_opens_with_system_turns_and_the_full_cast() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &[("Mira", false), ("Kael", false)]).await;
    seed_active_act(&pool, adventure.id, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &mut conn,
        &adventure,
        &low_closure_director(),
    )
    .await
    .expect("build prompt");

    // First turn is the system head, listing the whole cast.
    assert_eq!(turns[0].speaker, Speaker::System);
    assert!(turns[0].text.contains("## CHARACTERS CAST"));
    assert!(turns[0].text.contains("Mira."));
    assert!(turns[0].text.contains("Kael."));
}

#[tokio::test]
async fn system_head_renders_the_adventure_settings() {
    let (pool, _db) = setup_db().await;
    let mut adventure = setup_adventure(&pool, &[("Mira", false)]).await;
    adventure.title = "Tides of Glass".into();
    adventure.headline = "A drowned city".into();
    adventure.genre = Some("mystery".into());
    adventure.tone = "tense".into();
    adventure.lore = adventure.lore.with_text("The sea remembers.".into());
    seed_active_act(&pool, adventure.id, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &mut conn,
        &adventure,
        &low_closure_director(),
    )
    .await
    .expect("build prompt");

    let head = &turns[0].text;
    assert!(head.contains("## SETTINGS"));
    assert!(head.contains("Title: Tides of Glass"));
    assert!(head.contains("Premise: A drowned city"));
    assert!(head.contains("Genre: mystery"));
    assert!(head.contains("Tone: tense"));
    assert!(head.contains("Lore: The sea remembers."));
}

#[tokio::test]
async fn pacing_prompt_appears_with_counts_once_the_gate_opens() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &[("Mira", false)]).await;
    seed_active_act(&pool, adventure.id, None).await;
    let author = adventure.characters[0].clone();
    for i in 0..3 {
        seed_player_message(&pool, &adventure, &author, &format!("line {i}")).await;
    }

    let mut conn = pool.acquire().await.expect("acquire");
    // A small cap opens the pacing gate; 3 of ~5 messages is 60%.
    let eager = Director::new(DirectorSettings {
        act_max_messages: 5,
        ..Default::default()
    });
    let turns = ContextBuilder::build_autonomous_chat_prompt(&mut conn, &adventure, &eager)
        .await
        .expect("build prompt");

    let pacing = turns
        .iter()
        .find(|t| t.text.contains("## PACING"))
        .expect("a pacing turn once the gate opens");
    assert!(pacing.text.contains("message 3 of about 5"));
    assert!(pacing.text.contains("60%"));
}

#[tokio::test]
async fn previous_messages_classify_player_and_cast_turns() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &[("Hero", true), ("Mira", false)]).await;
    seed_active_act(&pool, adventure.id, None).await;

    let player = adventure.characters[0].clone();
    let npc = adventure.characters[1].clone();
    seed_player_message(&pool, &adventure, &player, "I open the door.").await;
    seed_cast_message(&pool, &adventure, &npc, "It creaks.").await;

    let mut conn = pool.acquire().await.expect("acquire");
    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &mut conn,
        &adventure,
        &low_closure_director(),
    )
    .await
    .expect("build prompt");

    let player_turn = turns
        .iter()
        .find(|t| t.speaker == Speaker::Player)
        .expect("a player turn");
    assert_eq!(player_turn.text, "I open the door.");

    let cast_turn = turns
        .iter()
        .find(|t| t.speaker == Speaker::Cast)
        .expect("a cast turn");
    assert_eq!(cast_turn.text, "Mira: It creaks.");
}

#[tokio::test]
async fn the_player_characters_line_is_a_player_turn() {
    // A message authored by the is_player character classifies as a player (user) turn.
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &[("Hero", true), ("Mira", false)]).await;
    seed_active_act(&pool, adventure.id, None).await;

    let player = adventure.characters[0].clone();
    seed_player_message(&pool, &adventure, &player, "I stare at the sky.").await;

    let mut conn = pool.acquire().await.expect("acquire");
    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &mut conn,
        &adventure,
        &low_closure_director(),
    )
    .await
    .expect("build prompt");

    let player_turn = turns
        .iter()
        .find(|t| t.speaker == Speaker::Player)
        .expect("the player character's line must be a player turn");
    assert_eq!(player_turn.text, "I stare at the sky.");
}

#[tokio::test]
async fn static_context_carries_the_running_digest_of_the_active_act() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &[("Mira", false)]).await;
    seed_active_act(&pool, adventure.id, Some("They escaped the keep.")).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let turns =
        ContextBuilder::build_narrator_chat_prompt(&mut conn, &adventure, &low_closure_director())
            .await
            .expect("build prompt");

    let joined = turns
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("### CURRENT ACT"));
    assert!(joined.contains("### THIS ACT SO FAR"));
    assert!(joined.contains("They escaped the keep."));
}

#[tokio::test]
async fn closure_prompt_appears_only_once_the_gate_opens() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &[("Mira", false)]).await;
    seed_active_act(&pool, adventure.id, None).await;
    let author = adventure.characters[0].clone();
    // Three messages in the act.
    for i in 0..3 {
        seed_player_message(&pool, &adventure, &author, &format!("line {i}")).await;
    }

    let mut conn = pool.acquire().await.expect("acquire");

    // A tiny cap (max 2 => gate at >= 1) opens the closure gate.
    let eager = Director::new(DirectorSettings {
        act_max_messages: 2,
        ..Default::default()
    });
    let turns = ContextBuilder::build_autonomous_chat_prompt(&mut conn, &adventure, &eager)
        .await
        .expect("build prompt");
    let has_closure = turns
        .iter()
        .any(|t| t.text.contains("## CLOSURE INFORMATION"));
    assert!(
        has_closure,
        "closure prompt should be present once the gate opens"
    );
    // The closure instruction asks the model to set the act_closure field.
    let closure = turns
        .iter()
        .find(|t| t.text.contains("## CLOSURE INFORMATION"))
        .expect("a closure turn");
    assert!(closure.text.contains("\"act_closure\""));

    // A huge cap keeps the gate shut.
    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &mut conn,
        &adventure,
        &low_closure_director(),
    )
    .await
    .expect("build prompt");
    let has_closure = turns
        .iter()
        .any(|t| t.text.contains("## CLOSURE INFORMATION"));
    assert!(
        !has_closure,
        "closure prompt should be absent while the gate is shut"
    );
}

// endregion

// region Distill prompt

#[tokio::test]
async fn distill_prompt_leads_with_the_distill_instruction_and_the_turns() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool, &[("Mira", false)]).await;
    seed_active_act(&pool, adventure.id, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let messages = vec![
        Message::new(
            adventure.id,
            None,
            None,
            "Hero".into(),
            "We fled.",
            false,
            vec![],
        ),
        Message::new(
            adventure.id,
            None,
            None,
            "Mira".into(),
            "They followed.",
            false,
            vec![],
        ),
    ];

    let turns = ContextBuilder::build_distill_prompt(&mut conn, &adventure, &messages)
        .await
        .expect("build distill prompt");

    assert_eq!(turns[0].speaker, Speaker::System);
    assert!(turns[0].text.contains("## DISTILL"));
    // The conversation turns are appended as player turns.
    let joined = turns
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("Hero: We fled."));
    assert!(joined.contains("Mira: They followed."));
}
// endregion
