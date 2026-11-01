use living_voices::backend::application::director::Director;
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::domain::message::Message;
use living_voices::config::{DirectorSettings, MessageThresholdRange};
use uuid::Uuid;

fn cast(names: &[&str]) -> Vec<Character> {
    names
        .iter()
        .map(|n| {
            Character::new(
                Uuid::new_v4(),
                (*n).to_string(),
                Uuid::new_v4(),
                "a diver",
                "",
                CharacterStatus::Present,
                false,
            )
        })
        .collect()
}

fn line(ooc: bool) -> Message {
    Message::new(
        Uuid::new_v4(),
        None,
        None,
        "Player".to_string(),
        "hello",
        ooc,
    )
}

fn narration() -> Message {
    Message::new(
        Uuid::new_v4(),
        None,
        Some(Uuid::new_v4()),
        "NARRATOR".to_string(),
        "the tide rises",
        false,
    )
}

fn director(targeted_pick_chance: f32, narrator_change_if_ooc: f32, ramp: (u8, u8)) -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance,
        narrator_change_if_ooc,
        act_max_messages: 200,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: ramp.0,
            max: ramp.1,
        },
    })
}

// ---------------------------------------------------------------- who speaks

#[test]
fn narrator_speaks_when_there_is_no_cast() {
    // given: a director and an empty cast
    let d = director(1.0, 0.0, (255, 255));
    // when/then: the narrator speaks, short-circuiting before any roll
    assert!(d.should_narrator_speak(&[], &line(false), 0));
}

#[test]
fn ooc_goes_to_the_narrator_only_when_its_chance_is_certain() {
    // given: a present cast and an out-of-character line
    let cast = cast(&["Mira"]);
    let ooc = line(true);

    // when: the ooc->narrator chance is certain -> then the narrator speaks
    let certain = director(1.0, 1.0, (255, 255));
    assert!(certain.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &ooc, 0));

    // when: the chance is zero -> then a character speaks instead
    let never = director(1.0, 0.0, (255, 255));
    assert!(!never.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &ooc, 0));
}

// ---------------------------------------------------------------- the ramp

#[test]
fn ramp_is_zero_below_min() {
    // given: a ramp over [20, 30]
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira"]);
    // when: 19 messages (below min) -> then the narrator never speaks
    assert!(!d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(false), 19));
}

#[test]
fn ramp_is_certain_at_or_after_max() {
    // given: a ramp over [20, 30]
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira"]);
    // when: at or past max -> then the narrator always speaks
    assert!(d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(false), 30));
    assert!(d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(false), 100));
}

// ------------------------------------------------------------ picking a character

#[test]
fn the_lone_candidate_is_always_picked() {
    // given: a single-member cast
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira"]);
    // when/then: that member is picked directly
    assert_eq!(
        d.pick_character(&cast.iter().collect::<Vec<_>>())
            .expect("picked")
            .name,
        "Mira"
    );
}

#[test]
fn a_targeted_pick_lands_on_one_of_the_candidates() {
    // given: a multi-member cast and a certain targeted-pick chance
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira", "Kael"]);
    let candidates = cast.iter().collect::<Vec<_>>();
    // when/then: the roll always fires, so one of the candidates is picked
    let picked = d.pick_character(&candidates).expect("picked");
    assert!(picked.name == "Mira" || picked.name == "Kael");
}

#[test]
fn multiple_candidates_defer_to_the_model() {
    // given: a cast with more than one member and no targeted-pick chance
    let d = director(0.0, 0.0, (20, 30));
    let cast = cast(&["Mira", "Kael"]);
    // when/then: with no roll firing, the pick is deferred to the model
    assert!(d.pick_character(&cast.iter().collect::<Vec<_>>()).is_none());
}

#[test]
fn an_empty_cast_has_nobody_to_pick() {
    // given: no candidates
    let d = director(1.0, 0.0, (20, 30));
    // when/then: there is nobody to pick
    assert!(d.pick_character(&[]).is_none());
}
