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
                "female".to_string(),
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

fn director(targeted_pick_chance: f32, narrator_change_if_ooc: f32, ramp: (u8, u8)) -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance,
        narrator_change_if_ooc,
        act_max_messages: 200,
        narrator_min_gap: 0,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: ramp.0,
            max: ramp.1,
        },
        ..Default::default()
    })
}

// ---------------------------------------------------------------- who speaks

#[test]
fn narrator_speaks_when_there_is_no_cast() {
    // given: a director and an empty cast
    let d = director(1.0, 0.0, (255, 255));
    // when/then: the narrator speaks, short-circuiting before any roll
    assert!(d.should_narrator_speak(&[], &line(false), Some(0)));
}

#[test]
fn ooc_goes_to_the_narrator_only_when_its_chance_is_certain() {
    // given: a present cast and an out-of-character line
    let cast = cast(&["Mira"]);
    let ooc = line(true);

    // when: the ooc->narrator chance is certain -> then the narrator speaks
    let certain = director(1.0, 1.0, (255, 255));
    assert!(certain.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &ooc, Some(0)));

    // when: the chance is zero -> then a character speaks instead
    let never = director(1.0, 0.0, (255, 255));
    assert!(!never.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &ooc, Some(0)));
}

// ---------------------------------------------------------------- the ramp

#[test]
fn ramp_is_zero_below_min() {
    // given: a ramp over [20, 30]
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira"]);
    // when: 19 messages (below min) -> then the narrator never speaks
    assert!(!d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(false), Some(19)));
}

#[test]
fn ramp_is_certain_at_or_after_max() {
    // given: a ramp over [20, 30]
    let d = director(1.0, 0.0, (20, 30));
    let cast = cast(&["Mira"]);
    // when: at or past max -> then the narrator always speaks
    assert!(d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(false), Some(30)));
    assert!(d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(false), Some(100)));
}

// ---------------------------------------------------------------- the narrator gap

fn gap_director(gap: u8) -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance: 1.0,
        narrator_change_if_ooc: 1.0,
        narrator_min_gap: gap,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: 0,
            max: 0,
        },
        ..Default::default()
    })
}

#[test]
fn narrator_stays_silent_inside_the_gap() {
    // given: a 10-message gap and a cast to speak for the narrator
    let d = gap_director(10);
    let cast = cast(&["Mira"]);
    let candidates = cast.iter().collect::<Vec<_>>();

    // when: fewer than 10 messages passed since the last narration -> then it stays silent,
    // whether the player's line is in character or out of character
    for since in 0..10 {
        assert!(!d.should_narrator_speak(&candidates, &line(false), Some(since)));
        assert!(!d.should_narrator_speak(&candidates, &line(true), Some(since)));
    }
}

#[test]
fn narrator_may_speak_once_the_gap_is_met() {
    // given: a 10-message gap
    let d = gap_director(10);
    let cast = cast(&["Mira"]);
    let candidates = cast.iter().collect::<Vec<_>>();

    // when: at or past the gap -> then the narrator is free to speak again
    assert!(d.should_narrator_speak(&candidates, &line(false), Some(10)));
    assert!(d.should_narrator_speak(&candidates, &line(false), Some(42)));
}

#[test]
fn a_narrator_that_never_spoke_is_not_gated() {
    // given: a 10-message gap and no narration anywhere in the adventure
    let d = gap_director(10);
    let cast = cast(&["Mira"]);

    // when/then: there is no previous narration to keep the gap from, so it may speak
    assert!(d.should_narrator_speak(&cast.iter().collect::<Vec<_>>(), &line(false), None));
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

// ------------------------------------------------------------ npc follow-ups

fn follow_up_director(chance: f32, max_chain: u8) -> Director {
    Director::new(DirectorSettings {
        follow_up_chance: chance,
        follow_up_max_chain: max_chain,
        ..Default::default()
    })
}

#[test]
fn no_follow_up_with_fewer_than_two_candidates() {
    // given: a certain chance but only one present cast member
    let d = follow_up_director(1.0, 3);
    let cast = cast(&["Mira"]);
    // when/then: there is nobody else to react, so no follow-up
    assert!(!d.should_a_conversation_follow_up(&cast.iter().collect::<Vec<_>>(), 0, &line(false)));
}

#[test]
fn no_follow_up_once_the_chain_cap_is_reached() {
    // given: a certain chance, two candidates, and a cap of 2
    let d = follow_up_director(1.0, 2);
    let cast = cast(&["Mira", "Kael"]);
    let candidates = cast.iter().collect::<Vec<_>>();
    // when/then: at or past the cap, the chain ends regardless of chance
    assert!(!d.should_a_conversation_follow_up(&candidates, 2, &line(false)));
    assert!(!d.should_a_conversation_follow_up(&candidates, 3, &line(false)));
}

#[test]
fn a_certain_chance_follows_up_within_the_cap() {
    // given: a certain chance, two candidates, room under the cap
    let d = follow_up_director(1.0, 3);
    let cast = cast(&["Mira", "Kael"]);
    // when/then: the first step always fires
    assert!(d.should_a_conversation_follow_up(&cast.iter().collect::<Vec<_>>(), 0, &line(false)));
}

#[test]
fn a_zero_chance_never_follows_up() {
    // given: no follow-up chance
    let d = follow_up_director(0.0, 3);
    let cast = cast(&["Mira", "Kael"]);
    // when/then: a follow-up never fires
    assert!(!d.should_a_conversation_follow_up(&cast.iter().collect::<Vec<_>>(), 0, &line(false)));
}
