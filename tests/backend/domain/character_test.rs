use living_voices::backend::domain::character::{Character, CharacterStatus};
use uuid::Uuid;

fn new_character() -> Character {
    Character::new(
        Uuid::new_v4(),
        "Maren".to_string(),
        Uuid::new_v4(),
        "A lighthouse keeper",
        "",
        CharacterStatus::Present,
        false,
    )
}

#[test]
fn new_sets_name_description_and_empty_fields() {
    let character = new_character();

    assert_eq!(character.name, "Maren");
    assert_eq!(character.headline, "A lighthouse keeper");
    assert_eq!(character.backstory, "");
    assert_eq!(character.appearance, None);
    assert_eq!(character.appearance_identity_prompt, None);
    assert_eq!(character.appearance_current_outfit, None);
    assert_eq!(character.appearance_current_pose, None);
    assert!(character.traits.is_empty());
    assert!(character.tags.is_empty());
    assert!(character.relations.is_empty());
}

#[test]
fn new_defaults_status_to_present() {
    let character = new_character();

    assert_eq!(character.status, CharacterStatus::default());
    assert_eq!(CharacterStatus::default().as_str(), "present");
}

#[test]
fn constructor_sets_backstory() {
    let character = Character::new(
        Uuid::new_v4(),
        "Maren".to_string(),
        Uuid::new_v4(),
        "A lighthouse keeper",
        "She kept the lamp lit for forty years.",
        CharacterStatus::Present,
        false,
    );

    assert_eq!(
        character.backstory,
        "She kept the lamp lit for forty years."
    );
}

#[test]
fn constructor_sets_status() {
    let character = Character::new(
        Uuid::new_v4(),
        "Maren".to_string(),
        Uuid::new_v4(),
        "A lighthouse keeper",
        "",
        CharacterStatus::Away,
        false,
    );

    assert_eq!(character.status, CharacterStatus::Away);
}

#[test]
fn add_tag_appends_tag() {
    let mut character = new_character();
    character.add_tag("protagonist".to_string());
    character.add_tag("brave".to_string());

    assert_eq!(character.tags, vec!["protagonist", "brave"]);
}

#[test]
fn remove_tag_drops_every_matching_entry() {
    let mut character = new_character();
    character.add_tag("brave".to_string());
    character.add_tag("brave".to_string());
    character.add_tag("lonely".to_string());

    character.remove_tag("brave".to_string());

    assert_eq!(character.tags, vec!["lonely"]);
}

#[test]
fn add_trait_appends_trait() {
    let mut character = new_character();
    character.add_trait("speaks only in tidal registers".to_string());

    assert_eq!(character.traits, vec!["speaks only in tidal registers"]);
}

#[test]
fn add_relation_appends_relation() {
    let mut character = new_character();
    character.add_relation("Mother".to_string());

    assert_eq!(character.relations, vec!["Mother".to_string()]);
}

#[test]
fn add_relation_appends_multiple_relations() {
    let mut character = new_character();
    character.add_relation("Sister".to_string());
    character.add_relation("Estranged sister".to_string());

    assert_eq!(character.relations.len(), 2);
    assert_eq!(character.relations[0], "Sister".to_string());
    assert_eq!(character.relations[1], "Estranged sister".to_string());
}

#[test]
fn new_generates_unique_ids() {
    assert_ne!(new_character().id, new_character().id);
}

#[test]
fn new_is_not_a_player_by_default() {
    assert!(!new_character().is_player);
}

#[test]
fn constructor_sets_is_player() {
    let character = Character::new(
        Uuid::new_v4(),
        "You".to_string(),
        Uuid::new_v4(),
        "",
        "",
        CharacterStatus::Present,
        true,
    );
    assert!(character.is_player);
}

#[test]
fn player_builds_the_default_player_character() {
    let adventure_id = Uuid::new_v4();
    let player = Character::new(
        Uuid::new_v4(),
        Character::DEFAULT_PLAYER_NAME.to_string(),
        adventure_id,
        "",
        "",
        CharacterStatus::Present,
        true,
    );

    assert!(player.is_player);
    assert_eq!(player.name, Character::DEFAULT_PLAYER_NAME);
    assert_eq!(player.adventure_id, adventure_id);
}

#[test]
fn status_as_str_covers_every_variant() {
    assert_eq!(CharacterStatus::Present.as_str(), "present");
    assert_eq!(CharacterStatus::Away.as_str(), "away");
    assert_eq!(CharacterStatus::Unavailable.as_str(), "unavailable");
}

#[test]
fn render_includes_name_description_backstory_traits_relations_and_appearance() {
    let mut character = Character::new(
        Uuid::new_v4(),
        "Maren".to_string(),
        Uuid::new_v4(),
        "A lighthouse keeper",
        "Kept the lamp lit for forty years.",
        CharacterStatus::Present,
        false,
    );
    character.add_trait("speaks softly".to_string());
    character.add_trait("never sleeps".to_string());
    character.add_relation("Son of the Duke".to_string());
    character.appearance = Some("Grey braid, oilskin coat.".to_string());

    let rendered = character.render();

    assert!(rendered.starts_with("### Maren\n"), "{rendered}");
    assert!(
        rendered.contains("*Headline*: A lighthouse keeper"),
        "{rendered}"
    );
    assert!(
        rendered.contains("*Backstory*: Kept the lamp lit for forty years."),
        "{rendered}"
    );
    assert!(
        rendered.contains("*Traits*: speaks softly, never sleeps."),
        "{rendered}"
    );
    assert!(
        rendered.contains("*Relations*: Son of the Duke"),
        "{rendered}"
    );
    assert!(
        rendered.contains("*Appearance*: Grey braid, oilskin coat."),
        "the prose period must not be doubled: {rendered}"
    );
    assert!(!rendered.contains(".."), "{rendered}");
}

#[test]
fn render_omits_empty_sections() {
    let rendered = new_character().render();

    assert!(rendered.starts_with("### Maren\n"), "{rendered}");
    assert!(
        rendered.contains("*Headline*: A lighthouse keeper"),
        "{rendered}"
    );
    assert!(!rendered.contains("*Backstory*"), "{rendered}");
    assert!(!rendered.contains("*Traits*"), "{rendered}");
    assert!(!rendered.contains("*Relations*"), "{rendered}");
    assert!(!rendered.contains("*Appearance*"), "{rendered}");
}

#[test]
fn render_puts_every_field_on_its_own_line() {
    let mut character = new_character();
    character.add_trait("speaks softly".to_string());
    character.add_relation("Son of Dragoon".to_string());

    let rendered = character.render();
    let lines: Vec<&str> = rendered.lines().collect();

    assert_eq!(lines[0], "### Maren", "{rendered}");
    assert!(lines.iter().all(|line| !line.ends_with(' ')), "{rendered}");
    assert!(
        lines.iter().any(|line| line.starts_with("*Traits*")),
        "{rendered}"
    );
    assert!(
        lines.iter().any(|line| line.starts_with("*Relations*")),
        "{rendered}"
    );
}

#[test]
fn render_skips_blank_entries_instead_of_leaving_an_empty_label() {
    let mut character = new_character();
    character.add_trait("   ".to_string());
    character.add_tag(String::new());
    character.add_relation("  Son of Dragoon  ".to_string());

    let rendered = character.render();

    assert!(!rendered.contains("*Traits*"), "{rendered}");
    assert!(
        rendered.contains("*Relations*: Son of Dragoon."),
        "a padded entry should be trimmed, not dropped: {rendered}"
    );
}

#[test]
fn with_message_example_sets_the_voice_samples() {
    let character = new_character().with_message_example("Maren: \"The lamp stays lit.\"");

    assert_eq!(
        character.message_example(),
        Some("Maren: \"The lamp stays lit.\"")
    );
}

#[test]
fn message_example_is_none_when_the_author_left_it_blank() {
    assert_eq!(new_character().message_example(), None);

    let padded = new_character().with_message_example("   \n  ");
    assert_eq!(padded.message_example(), None);
}

#[test]
fn message_example_is_trimmed_but_keeps_its_line_breaks() {
    let character = new_character().with_message_example("\n  Maren: one.\nYou: two.\n  ");

    assert_eq!(character.message_example(), Some("Maren: one.\nYou: two."));
}

#[test]
fn render_keeps_the_voice_samples_out_of_the_cast_line() {
    // The cast roster renders every character, so examples must not ride along.
    let character = new_character().with_message_example("Maren: \"The lamp stays lit.\"");

    let rendered = character.render();

    assert!(!rendered.contains("The lamp stays lit."), "{rendered}");
}

#[test]
fn with_appearance_sets_the_appearance() {
    let character = new_character()
        .with_appearance("A weathered woman with copper hair and a salt-stained coat.");

    assert_eq!(
        character.appearance.as_deref(),
        Some("A weathered woman with copper hair and a salt-stained coat.")
    );
}

#[test]
fn with_appearance_identity_prompt_sets_the_identity_prompt() {
    let character =
        new_character().with_appearance_identity_prompt("copper hair, grey eyes, wiry build");

    assert_eq!(
        character.appearance_identity_prompt.as_deref(),
        Some("copper hair, grey eyes, wiry build")
    );
}

#[test]
fn with_appearance_current_outfit_sets_the_outfit() {
    let character = new_character()
        .with_appearance_current_outfit("oilskin coat two sizes too big, sou'wester");

    assert_eq!(
        character.appearance_current_outfit.as_deref(),
        Some("oilskin coat two sizes too big, sou'wester")
    );
}

#[test]
fn with_appearance_current_pose_sets_the_current_pose() {
    let character =
        new_character().with_appearance_current_pose("reaching up to trim the lamp wick");

    assert_eq!(
        character.appearance_current_pose.as_deref(),
        Some("reaching up to trim the lamp wick")
    );
}
