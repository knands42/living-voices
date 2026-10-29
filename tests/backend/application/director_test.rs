use living_voices::backend::application::director::Director;
use living_voices::backend::domain::character::Character;
use living_voices::backend::domain::message::Message;
use living_voices::config::{DirectorSettings, MessageThresholdRange};
use uuid::Uuid;

fn cast(names: &[&str]) -> Vec<Character> {
    names
        .iter()
        .map(|n| Character::new((*n).to_string(), None, "a diver".into()))
        .collect()
}

fn line(targets: &[&Character], ooc: bool) -> Message {
    let targeted = if targets.is_empty() {
        None
    } else {
        Some(targets.iter().map(|c| (*c).clone()).collect())
    };
    Message::new(
        Uuid::new_v4(),
        None,
        None,
        "Player".to_string(),
        "hello",
        ooc,
        targeted.unwrap_or(vec![]),
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
        vec![],
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
    assert!(d.should_narrator_speak(&[], &line(&[], false), 0));
}

#[test]
fn ooc_goes_to_the_narrator_only_when_its_chance_is_certain() {
    // given: a present cast and an out-of-character line
    let cast = cast(&["Mira"]);
    let ooc = line(&[], true);

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
    assert!(!d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(&[], false), 19));
}

#[test]
fn ramp_is_certain_at_or_after_max() {
    // given: a ramp over [20, 30]
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira"]);
    // when: at or past max -> then the narrator always speaks
    assert!(d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(&[], false), 30));
    assert!(d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(&[], false), 100));
}

// ---------------------------------------------------------------- targeting

#[test]
fn a_targeted_message_picks_that_character_when_chance_is_certain() {
    // given: a line aimed at Kael and a certain targeting chance
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira", "Kael"]);
    let incoming = line(&[&cast[1]], false);
    // when/then: Kael is picked
    assert_eq!(
        d.pick_character(&cast.iter().collect::<Vec<_>>(), &incoming)
            .expect("picked")
            .name,
        "Kael"
    );
}

#[test]
fn targeting_defers_to_the_model_when_chance_is_zero() {
    // given: a targeted line but a zero targeting chance
    let d = director(0.0, 0.0, (20, 30));
    let cast = cast(&["Mira", "Kael"]);
    let incoming = line(&[&cast[1]], false);
    // when/then: the pick is deferred to the model
    assert!(
        d.pick_character(&cast.iter().collect::<Vec<_>>(), &incoming)
            .is_none()
    );
}

#[test]
fn an_untargeted_message_defers_even_when_chance_is_certain() {
    // given: an untargeted line with a certain targeting chance
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira", "Kael"]);
    let incoming = line(&[], false);
    // when/then: with nobody targeted, there is nobody to pick directly
    assert!(
        d.pick_character(&cast.iter().collect::<Vec<_>>(), &incoming)
            .is_none()
    );
}

#[test]
fn a_target_not_in_the_cast_defers() {
    // given: a line aimed at someone absent from the cast
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira", "Laura"]);
    let outsider = Character::new("Nobody".to_string(), None, "a stranger".into());
    let incoming = line(&[&outsider], false);
    // when/then: the pick is deferred
    assert!(
        d.pick_character(&cast.iter().collect::<Vec<_>>(), &incoming)
            .is_none()
    );
}

#[test]
fn a_target_is_always_selected_if_only_one() {
    // given: a line aimed at someone
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira"]);
    let outsider = Character::new("Nobody".to_string(), None, "a stranger".into());
    let incoming = line(&[&outsider], false);
    // when/then: Mira is picked even the target character does not belong to the cast
    assert_eq!(
        d.pick_character(&cast.iter().collect::<Vec<_>>(), &incoming)
            .expect("picked")
            .name,
        "Mira"
    );
}
