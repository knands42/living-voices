use chrono::Utc;
use living_voices::backend::application::character_gallery::CharacterGallery;
use living_voices::backend::domain::adventure::{Adventure, AdventureStatus};
use living_voices::backend::domain::character::{Character, CharacterStatus, ImageFraming};
use living_voices::backend::domain::lore::Lore;
use living_voices::backend::domain::narrator::Narrator;
use uuid::Uuid;

fn adventure(id: Uuid) -> Adventure {
    Adventure::new(
        id,
        "Salt & Ash",
        "A drowned city",
        Narrator::new(Uuid::new_v4(), id, "The Keeper", "hushed"),
        "tense",
        Lore::new(Uuid::new_v4(), ""),
        AdventureStatus::Draft,
        Utc::now(),
        Utc::now(),
    )
}

fn character(adventure_id: Uuid) -> Character {
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
}

#[test]
fn prefix_numbers_the_first_shot_zero_and_counts_up_from_there() {
    // given a character directory with nothing in it yet
    let root = std::env::temp_dir().join(format!("living_voices_gallery_{}", Uuid::new_v4()));
    let adventure = adventure(Uuid::new_v4());
    let character = character(adventure.id);
    let gallery = CharacterGallery::new(&root);
    let dir = root.join("salt_ash/mara_venn");
    std::fs::create_dir_all(&dir).expect("create gallery dir");

    // when the first portrait has not been made yet
    let first = gallery.prefix(&adventure.title, &character, ImageFraming::Portrait);

    // then it is shot zero, the next one counts up, and other framings start over
    assert_eq!(first, "salt_ash/mara_venn/mara_venn_portrait_0");

    std::fs::write(dir.join("mara_venn_portrait_0_00001_.png"), b"original").expect("write shot");

    assert_eq!(
        gallery.prefix(&adventure.title, &character, ImageFraming::Portrait),
        "salt_ash/mara_venn/mara_venn_portrait_1"
    );
    assert_eq!(
        gallery.prefix(&adventure.title, &character, ImageFraming::FullBody),
        "salt_ash/mara_venn/mara_venn_full_body_0"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn latest_takes_the_highest_shot_number_the_prefix_produced() {
    // given two portraits and a pose, numbered past nine so text order would mislead
    let root = std::env::temp_dir().join(format!("living_voices_gallery_{}", Uuid::new_v4()));
    let adventure = adventure(Uuid::new_v4());
    let character = character(adventure.id);
    let gallery = CharacterGallery::new(&root);
    let dir = root.join("salt_ash/mara_venn");
    std::fs::create_dir_all(&dir).expect("create gallery dir");
    std::fs::write(dir.join("mara_venn_portrait_2_00001_.png"), b"older").expect("write older");
    std::fs::write(dir.join("mara_venn_portrait_10_00001_.png"), b"newer").expect("write newer");
    std::fs::write(dir.join("mara_venn_pose_1_00001_.png"), b"pose").expect("write pose");

    // when
    let latest = gallery.latest(&adventure.title, &character, ImageFraming::Portrait);

    // then
    assert_eq!(latest, Some(dir.join("mara_venn_portrait_10_00001_.png")));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn latest_is_empty_until_the_shot_was_queued() {
    // given
    let root = std::env::temp_dir().join(format!("living_voices_gallery_{}", Uuid::new_v4()));
    let adventure = adventure(Uuid::new_v4());
    let character = character(adventure.id);
    let gallery = CharacterGallery::new(&root);

    // when
    let latest = gallery.latest(&adventure.title, &character, ImageFraming::Portrait);

    // then
    assert_eq!(latest, None);
}
