use sqlx::{SqliteConnection, sqlite::SqlitePool};

use crate::backend::application::helper::{setup_db, start_adventure_with_default_cast};
use living_voices::backend::application::context::{ContextBuilder, ContextData};
use living_voices::backend::application::director::Director;
use living_voices::backend::application::port::chat_port::{PromptTurn, Speaker};
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::{Character, CharacterStatus, ImageFraming};
use living_voices::backend::domain::memory::{Fact, Permanence};
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::fact_persistence::upsert_facts;
use living_voices::config::DirectorSettings;
use uuid::Uuid;

// region Autonomous prompt

#[tokio::test]
async fn autonomous_prompt_opens_with_system_turns_and_the_full_cast() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");
    let mut conn = pool.acquire().await.expect("acquire");
    let data = context_data(&mut conn, &adventure).await;

    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &adventure,
        &low_closure_director(),
        &data,
        &[],
    );

    // First turn is the system head, listing the whole cast under one heading each.
    assert_eq!(turns[0].speaker, Speaker::System);
    assert!(turns[0].text.contains("## CHARACTERS CAST"));
    assert!(turns[0].text.contains("### Mira"), "{}", turns[0].text);
    assert!(turns[0].text.contains("### Kael"), "{}", turns[0].text);
}

#[tokio::test]
async fn system_head_renders_the_adventure_settings() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let mut adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");
    adventure.title = "Tides of Glass".into();
    adventure.headline = "A drowned city".into();
    adventure.tone = "tense".into();
    adventure.lore = adventure.lore.with_text("The sea remembers.".into());
    let mut conn = pool.acquire().await.expect("acquire");
    let data = context_data(&mut conn, &adventure).await;

    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &adventure,
        &low_closure_director(),
        &data,
        &[],
    );

    let head = &turns[0].text;
    assert!(head.contains("## SETTINGS"));
    assert!(head.contains("Title: Tides of Glass"));
    assert!(head.contains("Premise: A drowned city"));
    assert!(head.contains("Tone: tense"));
    assert!(head.contains("Lore: The sea remembers."));
}

#[tokio::test]
async fn pacing_prompt_appears_with_counts_once_the_gate_opens() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");
    let player = adventure
        .characters
        .iter()
        .find(|c| c.is_player)
        .expect("Mira to exist");

    for i in 0..3 {
        seed_player_message(&pool, &adventure, &player, &format!("line {i}")).await;
    }

    let mut conn = pool.acquire().await.expect("acquire");
    let eager_director = Director::new(DirectorSettings {
        act_max_messages: 5,
        ..Default::default()
    });
    let messages = last_messages(&mut conn, &adventure).await;
    let data = context_data(&mut conn, &adventure).await;
    let turns =
        ContextBuilder::build_autonomous_chat_prompt(&adventure, &eager_director, &data, &messages);

    let pacing = turns
        .iter()
        .find(|t| t.text.contains("## PACING"))
        .expect("a pacing turn once the gate opens");

    assert!(pacing.text.contains("message 3 of about 5"));
    assert!(pacing.text.contains("60%"));
    assert!(pacing.text.contains("Goal progress last judged: 0 of 5."));
}

#[tokio::test]
async fn previous_messages_classify_player_and_cast_turns() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");

    let player = adventure.characters[0].clone();
    let npc = adventure.characters[1].clone();
    seed_player_message(&pool, &adventure, &player, "I open the door.").await;
    seed_cast_message(&pool, &adventure, &npc, "It creaks.").await;

    let mut conn = pool.acquire().await.expect("acquire");
    let messages = last_messages(&mut conn, &adventure).await;
    let data = context_data(&mut conn, &adventure).await;
    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &adventure,
        &low_closure_director(),
        &data,
        &messages,
    );

    let player_turn = turns
        .iter()
        .find(|t| t.speaker == Speaker::Player)
        .expect("a player turn");
    assert_eq!(player_turn.text, "I open the door.");

    let cast_turn = turns
        .iter()
        .find(|t| t.speaker == Speaker::Cast)
        .expect("a cast turn");
    assert_eq!(
        cast_turn.text,
        r#"{"act_closure":null,"replier":"Mira","reply":"It creaks."}"#
    );
}

#[tokio::test]
async fn the_player_characters_line_is_a_player_turn() {
    // A message authored by the is_player character classifies as a player (user) turn.
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");

    let player = adventure.characters[0].clone();
    seed_player_message(&pool, &adventure, &player, "I stare at the sky.").await;

    let mut conn = pool.acquire().await.expect("acquire");
    let messages = last_messages(&mut conn, &adventure).await;
    let data = context_data(&mut conn, &adventure).await;
    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &adventure,
        &low_closure_director(),
        &data,
        &messages,
    );

    let player_turn = turns
        .iter()
        .find(|t| t.speaker == Speaker::Player)
        .expect("the player character's line must be a player turn");
    assert_eq!(player_turn.text, "I stare at the sky.");
}

#[tokio::test]
async fn static_context_carries_the_running_digest_of_the_active_act() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");

    // Set the running_digest on the active act created by start_adventure()
    use living_voices::backend::persistence::act_persistence::{
        get_active_act, update_act_running_digest,
    };
    let mut conn = pool.acquire().await.expect("acquire");
    let active_act = get_active_act(&mut conn, adventure.id)
        .await
        .expect("get active act")
        .expect("has active act");
    update_act_running_digest(&mut conn, active_act.id, "They escaped the keep.", 0)
        .await
        .expect("update running_digest");
    let data = context_data(&mut conn, &adventure).await;
    let turns =
        ContextBuilder::build_narrator_chat_prompt(&adventure, &low_closure_director(), &data, &[]);

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
async fn static_context_renders_the_fact_snapshot() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");

    let mut conn = pool.acquire().await.expect("acquire");
    let fact = Fact::new("mira", "fears deep water", Permanence::Permanent);
    upsert_facts(&mut conn, adventure.id, &[fact])
        .await
        .expect("upsert fact");

    let data = context_data(&mut conn, &adventure).await;
    let turns =
        ContextBuilder::build_narrator_chat_prompt(&adventure, &low_closure_director(), &data, &[]);

    let joined = turns
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("### FACTS"));
    assert!(joined.contains("mira: fears deep water -> permanent"));
}

#[tokio::test]
async fn closure_prompt_appears_only_once_the_gate_opens() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");
    let player = adventure
        .characters
        .iter()
        .find(|c| c.is_player)
        .expect("player character to exist");

    // Three messages in the act.
    for i in 0..3 {
        seed_player_message(&pool, &adventure, &player, &format!("line {i}")).await;
    }

    let mut conn = pool.acquire().await.expect("acquire");

    // A tiny cap (max 2 => gate at >= 1) opens the closure gate.
    let eager = Director::new(DirectorSettings {
        act_max_messages: 2,
        ..Default::default()
    });
    let messages = last_messages(&mut conn, &adventure).await;
    let data = context_data(&mut conn, &adventure).await;
    let turns = ContextBuilder::build_autonomous_chat_prompt(&adventure, &eager, &data, &messages);
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
        &adventure,
        &low_closure_director(),
        &data,
        &messages,
    );
    let has_closure = turns
        .iter()
        .any(|t| t.text.contains("## CLOSURE INFORMATION"));
    assert!(
        !has_closure,
        "closure prompt should be absent while the gate is shut"
    );
}

// endregion

// region Character voice examples

#[tokio::test]
async fn single_character_history_renders_cast_turns_as_the_envelope() {
    // given a prior cast line in the transcript (the shape that used to teach the
    // model a bare "Name:"/"NARRATOR:" prefix it then leaked into the stored body)
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");

    let player = adventure.characters[0].clone();
    let npc = adventure.characters[1].clone();
    seed_player_message(&pool, &adventure, &player, "I open the door.").await;
    seed_cast_message(&pool, &adventure, &npc, "It creaks.").await;

    let mut conn = pool.acquire().await.expect("acquire");
    let messages = last_messages(&mut conn, &adventure).await;
    let data = context_data(&mut conn, &adventure).await;

    // when the single-character prompt is built
    let turns =
        ContextBuilder::build_single_character_chat_prompt(&adventure, &data, &npc, &messages);

    // then the cast turn is the {replier, reply} envelope, with no act_closure and
    // no bare "Name:" prefix carried in the body
    let cast_turn = turns
        .iter()
        .find(|t| t.speaker == Speaker::Cast)
        .expect("a cast turn");
    assert_eq!(cast_turn.text, r#"{"replier":"Mira","reply":"It creaks."}"#);
    assert!(
        !cast_turn.text.contains("act_closure"),
        "the character envelope must not carry act_closure: {}",
        cast_turn.text
    );
}

#[tokio::test]
async fn single_character_instructions_forbid_a_name_prefix_and_show_the_envelope() {
    // given the single-character prompt head
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");
    let npc = adventure.characters[1].clone();
    let mut conn = pool.acquire().await.expect("acquire");
    let data = context_data(&mut conn, &adventure).await;

    // when
    let turns = ContextBuilder::build_single_character_chat_prompt(&adventure, &data, &npc, &[]);
    let head = &turns[0].text;

    // then the contract is spelled out: one JSON envelope, the prefix is banned,
    // and two worked examples ride along — both the named character, never the narrator
    assert!(head.contains("## REPLY FORMAT"), "{head}");
    assert!(head.contains("\"replier\""), "{head}");
    assert!(head.contains("\"reply\""), "{head}");
    assert!(
        head.contains("never start it with \"NARRATOR:\""),
        "the prefix ban must be explicit: {head}"
    );
    assert!(
        !head.contains("\"replier\": \"NARRATOR\""),
        "the single-character prompt never offers NARRATOR as a replier: {head}"
    );
    assert_eq!(
        head.matches("{\"replier\":").count(),
        2,
        "two envelope examples, like the autonomous prompt: {head}"
    );
}

#[test]
fn single_character_prompt_carries_the_voice_examples() {
    // given a character with voice samples, and no database involved
    let adventure = bare_adventure();
    let character = Character::new(
        Uuid::new_v4(),
        "Maren".into(),
        "female".to_string(),
        adventure.id,
        "A lighthouse keeper",
        "",
        CharacterStatus::Present,
        false,
    )
    .with_message_example("Maren: \"The lamp stays lit.\"\nYou: \"Even tonight?\"");

    // when
    let turns = ContextBuilder::build_single_character_chat_prompt(
        &adventure,
        &empty_context(),
        &character,
        &[],
    );

    // then the samples ride at the end of the system head, under their own heading
    let head = &turns[0].text;
    assert!(head.contains("## VOICE"), "{head}");
    assert!(head.contains("Maren: \"The lamp stays lit.\""), "{head}");
    assert!(
        head.find("## CHARACTER").unwrap() < head.find("## VOICE").unwrap(),
        "examples belong after the character block: {head}"
    );
}

#[test]
fn a_character_without_examples_gets_no_heading() {
    // given a character the author never wrote samples for
    let adventure = bare_adventure();
    let character = Character::new(
        Uuid::new_v4(),
        "Kael".into(),
        "female".to_string(),
        adventure.id,
        "A ferryman",
        "",
        CharacterStatus::Present,
        false,
    );

    // when
    let turns = ContextBuilder::build_single_character_chat_prompt(
        &adventure,
        &empty_context(),
        &character,
        &[],
    );

    // then
    assert!(!turns[0].text.contains("## VOICE"), "{}", turns[0].text);
}

#[test]
fn the_cast_roster_never_renders_voice_examples() {
    // given an adventure whose cast all carry samples
    let mut adventure = bare_adventure();
    adventure.add_character(
        Character::new(
            Uuid::new_v4(),
            "Maren".into(),
            "female".to_string(),
            adventure.id,
            "A lighthouse keeper",
            "",
            CharacterStatus::Present,
            false,
        )
        .with_message_example("Maren: \"The lamp stays lit.\""),
    );

    // when
    let turns = ContextBuilder::build_autonomous_chat_prompt(
        &adventure,
        &low_closure_director(),
        &empty_context(),
        &[],
    );

    // then one copy per cast member would flood the prompt, so it stays out
    assert!(turns[0].text.contains("## CHARACTERS CAST"));
    assert!(
        !turns[0].text.contains("The lamp stays lit."),
        "{}",
        turns[0].text
    );
    assert!(!turns[0].text.contains("## VOICE"), "{}", turns[0].text);
}

// endregion

// region Character prompt fixtures

fn closed_act(
    adventure_id: Uuid,
    ordinal: u16,
    goal: &str,
    digest: &str,
) -> living_voices::backend::domain::memory::Act {
    use living_voices::backend::domain::memory::ActStatus;

    let mut act = living_voices::backend::domain::memory::Act::new(adventure_id, ordinal, goal);
    act.status = ActStatus::Resolved;
    act.index_line = Some(format!("Act {ordinal} — {goal}."));
    act.digest = Some(digest.to_string());
    act
}

#[test]
fn only_the_last_two_closed_acts_keep_their_digest() {
    // given four closed acts, each with an index line and a digest
    let adventure = bare_adventure();
    let acts = (1..=4)
        .map(|ordinal| closed_act(adventure.id, ordinal, "a goal", "the long digest of it"))
        .collect();

    let mut data = empty_context();
    data.acts = acts;

    let character = Character::new(
        Uuid::new_v4(),
        "Maren".into(),
        "female".to_string(),
        adventure.id,
        "A lighthouse keeper",
        "",
        CharacterStatus::Present,
        false,
    );

    // when
    let turns =
        ContextBuilder::build_single_character_chat_prompt(&adventure, &data, &character, &[]);

    // then the two older acts collapse to their single index line
    let context = turns
        .iter()
        .find(|t| t.text.contains("### PAST ACTS"))
        .expect("a PAST ACTS block")
        .text
        .clone();
    let past_acts = context
        .split("### PAST ACTS")
        .nth(1)
        .expect("the block body");

    for ordinal in 1..=2 {
        assert!(
            past_acts.contains(&format!("Act {ordinal} — a goal.")),
            "act {ordinal} should keep its index line: {past_acts}"
        );
    }
    let older = past_acts.split("Act 3").next().expect("the older acts");
    assert!(!older.contains("the long digest of it"), "{older}");

    // and only the two most recent ones still carry their digest
    for ordinal in 3..=4 {
        assert!(
            past_acts.contains(&format!("Act {ordinal}\ngoal:")),
            "act {ordinal} should keep its digest entry: {past_acts}"
        );
    }
    assert_eq!(
        past_acts.matches("the long digest of it").count(),
        2,
        "{past_acts}"
    );
}

fn bare_adventure() -> Adventure {
    use living_voices::backend::domain::adventure::AdventureStatus;
    use living_voices::backend::domain::lore::Lore;
    use living_voices::backend::domain::narrator::Narrator;

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    Adventure::new(
        id,
        "A drowned city",
        "The tide keeps what it takes.",
        Narrator::new(Uuid::new_v4(), id, "Narrator", "tense"),
        "tense",
        Lore::new(Uuid::new_v4(), ""),
        AdventureStatus::Draft,
        now,
        now,
    )
}

fn empty_context() -> ContextData {
    ContextData {
        acts: Vec::new(),
        facts: Vec::new(),
        state: None,
        messages_in_act: 0,
    }
}

// endregion

// region Distill prompt

#[tokio::test]
async fn distill_prompt_leads_with_the_distill_instruction_and_the_turns() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");
    let mut conn = pool.acquire().await.expect("acquire");

    let messages = vec![
        Message::new(adventure.id, None, None, "Hero".into(), "We fled.", false),
        Message::new(
            adventure.id,
            None,
            None,
            "Mira".into(),
            "They followed.",
            false,
        ),
    ];

    let data = context_data(&mut conn, &adventure).await;
    let turns = ContextBuilder::build_distill_prompt(&data, &messages);

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

// region Image prompt composition

fn mara(adventure_id: Uuid) -> Character {
    Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        "female".to_string(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    )
    .with_appearance("Copper hair, grey eyes, a salt-stained coat")
}

fn joined(turns: &[PromptTurn]) -> String {
    turns
        .iter()
        .map(|turn| turn.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn identity_prompt_asks_for_identity_only_and_hands_over_the_author_description() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");
    let character = mara(adventure.id);

    let turns = ContextBuilder::build_identity_char_prompt(&adventure, &character, vec![])
        .expect("identity prompt");
    let prompt = joined(&turns);

    assert!(turns[0].text.contains("## CHARACTER IDENTITY"));
    assert!(prompt.contains("Mara Venn"));
    assert!(prompt.contains("Copper hair, grey eyes, a salt-stained coat"));
    // Identity never frames the shot: framing is decided when the image is asked for.
    assert!(!prompt.to_lowercase().contains("head-and-shoulders"));
}

#[tokio::test]
async fn outfit_prompt_carries_the_scene_messages() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("adventure to exist on the snapshot");
    let character = mara(adventure.id);
    let messages = vec![Message::new(
        adventure.id,
        Some(character.id),
        None,
        "Mara Venn".to_string(),
        "*throws the soaked coat onto the fire*".to_string(),
        false,
    )];

    let turns = ContextBuilder::build_outfit_char_prompt(&adventure, &character, vec![], &messages)
        .expect("outfit prompt");
    let prompt = joined(&turns);

    assert!(turns[0].text.contains("## CHARACTER CURRENT OUTFIT"));
    assert!(prompt.contains("*throws the soaked coat onto the fire*"));
}

#[test]
fn composed_prompt_stacks_framing_identity_outfit_and_pose() {
    let mut character = mara(Uuid::new_v4())
        .with_appearance_identity_prompt("copper hair, grey eyes, wiry build")
        .with_appearance_current_outfit("oilskin coat two sizes too big")
        .with_appearance_current_pose("reaching up to trim the lamp wick");

    let pose =
        ContextBuilder::compose_character_image_prompt(&character, ImageFraming::FullBodyPose)
            .expect("composed prompt");

    assert!(pose.starts_with("full body"));
    assert!(pose.contains("copper hair, grey eyes, wiry build"));
    assert!(pose.contains("oilskin coat two sizes too big"));
    assert!(pose.contains("reaching up to trim the lamp wick"));

    let full_body =
        ContextBuilder::compose_character_image_prompt(&character, ImageFraming::FullBody)
            .expect("composed prompt");

    assert!(full_body.starts_with("full body"));
    assert!(full_body.contains("oilskin coat two sizes too big"));
    assert!(!full_body.contains("reaching up to trim the lamp wick"));

    // The pose layer describes a whole-body action, so a portrait drops it.
    character.appearance_current_pose = Some("reaching up to trim the lamp wick".into());
    let portrait =
        ContextBuilder::compose_character_image_prompt(&character, ImageFraming::Portrait)
            .expect("composed prompt");

    assert!(portrait.starts_with("head and shoulders portrait"));
    assert!(portrait.contains("oilskin coat two sizes too big"));
    assert!(!portrait.contains("reaching up to trim the lamp wick"));
}

#[test]
fn composed_prompt_falls_back_to_the_author_description() {
    let character = mara(Uuid::new_v4());

    let prompt = ContextBuilder::compose_character_image_prompt(&character, ImageFraming::Portrait)
        .expect("composed prompt");

    assert!(prompt.contains("Copper hair, grey eyes, a salt-stained coat"));
}

#[test]
fn composed_prompt_is_none_without_any_visual_source() {
    let character = Character::new(
        Uuid::new_v4(),
        "Nameless".to_string(),
        "female".to_string(),
        Uuid::new_v4(),
        "",
        "",
        CharacterStatus::Present,
        false,
    );

    assert!(
        ContextBuilder::compose_character_image_prompt(&character, ImageFraming::FullBody)
            .is_none()
    );
}

// endregion

// region DB fixtures

async fn seed_player_message(
    pool: &SqlitePool,
    adventure: &Adventure,
    author: &Character,
    text: &str,
) {
    use living_voices::backend::persistence::message_persistence::insert_message;
    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure.id,
        Some(author.id),
        None,
        author.name.clone(),
        text,
        false,
    );
    insert_message(&mut conn, &message).await.expect("insert");
}

async fn seed_cast_message(
    pool: &SqlitePool,
    adventure: &Adventure,
    author: &Character,
    text: &str,
) {
    use living_voices::backend::persistence::message_persistence::insert_message;
    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure.id,
        Some(author.id),
        None,
        author.name.clone(),
        text,
        false,
    );
    insert_message(&mut conn, &message).await.expect("insert");
}

async fn last_messages(conn: &mut SqliteConnection, adventure: &Adventure) -> Vec<Message> {
    use living_voices::backend::persistence::message_persistence::fetch_last_n;
    fetch_last_n(conn, adventure.id, 50)
        .await
        .expect("fetch messages")
}

async fn context_data(conn: &mut SqliteConnection, adventure: &Adventure) -> ContextData {
    ContextData::load(conn, adventure.id)
        .await
        .expect("load context data")
}

fn low_closure_director() -> Director {
    // A huge act cap so the closure gate never fires in these fixtures.
    Director::new(DirectorSettings {
        act_max_messages: 10_000,
        ..Default::default()
    })
}

// endregion
