use uuid::Uuid;

use living_voices::backend::domain::memory::Episode;

#[test]
fn episode_new_sets_fields() {
    let adv = Uuid::new_v4();
    let ep = Episode::new(
        adv,
        "Millbrook tavern",
        "They met the informant.",
        7,
        vec!["elara".to_string(), "vault".to_string()],
        10,
        24,
    );
    assert_ne!(ep.id, Uuid::nil());
    assert_eq!(ep.adventure_id, adv);
    assert!(ep.act_id.is_none());
    assert_eq!(ep.scene, "Millbrook tavern");
    assert_eq!(ep.importance, 7);
    assert_eq!(ep.entities, vec!["elara".to_string(), "vault".to_string()]);
    assert_eq!(ep.from_seq, 10);
    assert_eq!(ep.to_seq, 24);
}

#[test]
fn episode_render_lists_each_episode_under_a_heading() {
    let adv = Uuid::new_v4();
    let episodes = vec![
        Episode::new(adv, "harbor", "The ship sank.", 6, vec![], 1, 5),
        Episode::new(
            adv,
            "shore",
            "Survivors reached the shore.",
            4,
            vec![],
            6,
            9,
        ),
    ];
    let rendered = Episode::render(&episodes).expect("some output");
    assert!(rendered.starts_with("Story so far: "));
    assert!(rendered.contains("\n- [harbor] The ship sank."));
    assert!(rendered.contains("\n- [shore] Survivors reached the shore."));
}
