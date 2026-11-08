use std::path::{Path, PathBuf};

use crate::backend::domain::character::{Character, ImageFraming};

/// Where the character images live and how they are named.
#[derive(Clone, Debug)]
pub struct CharacterGallery {
    root: PathBuf,
}

impl CharacterGallery {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    // TODO: Get only the correct parameters here needed, adventure_title and character_name
    pub fn prefix(
        &self,
        adventure_title: &str,
        character: &Character,
        framing: ImageFraming,
    ) -> String {
        let adventure = slug(adventure_title);
        let character = slug(&character.name);
        format!("{adventure}/{character}/{character}_{}", shot(framing))
    }

    pub fn latest(
        &self,
        adventure_title: &str,
        character: &Character,
        framing: ImageFraming,
    ) -> Option<PathBuf> {
        let prefix = self.prefix(adventure_title, character, framing);
        let stem = Path::new(&prefix)
            .file_name()?
            .to_string_lossy()
            .into_owned();
        let dir = self.root.join(prefix).parent()?.to_path_buf();

        std::fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
            .filter(|path| {
                path.file_stem()
                    .is_some_and(|name| name.to_string_lossy().starts_with(&stem))
            })
            .max_by_key(|path| path.file_name().map(|name| name.to_os_string()))
    }
}

fn shot(framing: ImageFraming) -> &'static str {
    match framing {
        ImageFraming::Portrait => "portrait",
        ImageFraming::FullBody => "full_body",
        ImageFraming::FullBodyPose => "pose_1",
    }
}

fn slug(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() {
            slug.push(ch);
        } else if !slug.is_empty() && !slug.ends_with('_') {
            slug.push('_');
        }
    }

    let slug = slug.trim_end_matches('_');
    if slug.is_empty() {
        "unnamed".to_string()
    } else {
        slug.to_string()
    }
}
