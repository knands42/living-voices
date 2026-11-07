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
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    )
}

#[test]
fn prefix_puts_the_shot_under_the_adventure_and_character() {
    // given
    let adventure = adventure(Uuid::new_v4());
    let character = character(adventure.id);
    let gallery = CharacterGallery::new("/comfy/output");

    // when
    let portrait = gallery.prefix(&adventure, &character, ImageFraming::Portrait);
    let pose = gallery.prefix(&adventure, &character, ImageFraming::FullBodyPose);

    // then
    assert_eq!(portrait, "salt_ash/mara_venn/mara_venn_portrait");
    assert_eq!(pose, "salt_ash/mara_venn/mara_venn_pose_1");
}

#[test]
fn latest_takes_the_newest_file_the_prefix_produced() {
    // given
    let root = std::env::temp_dir().join(format!("living_voices_gallery_{}", Uuid::new_v4()));
    let adventure = adventure(Uuid::new_v4());
    let character = character(adventure.id);
    let gallery = CharacterGallery::new(&root);
    let dir = root.join("salt_ash/mara_venn");
    std::fs::create_dir_all(&dir).expect("create gallery dir");
    std::fs::write(dir.join("mara_venn_portrait_00001_.png"), b"first").expect("write first");
    std::fs::write(dir.join("mara_venn_portrait_00002_.png"), b"second").expect("write second");
    std::fs::write(dir.join("mara_venn_pose_1_00001_.png"), b"pose").expect("write pose");

    // when
    let latest = gallery.latest(&adventure, &character, ImageFraming::Portrait);

    // then
    assert_eq!(latest, Some(dir.join("mara_venn_portrait_00002_.png")));

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
    let latest = gallery.latest(&adventure, &character, ImageFraming::Portrait);

    // then
    assert_eq!(latest, None);
}
