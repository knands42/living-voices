use std::ffi::OsStr;
use std::path::PathBuf;

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

    /// The prefix of the shot about to be generated -> "<adventure_name>/<char_name>/<char_name>_<shot_type>_<number>")
    pub fn prefix(
        &self,
        adventure_title: &str,
        character: &Character,
        framing: ImageFraming,
    ) -> String {
        let next = self
            .numbered_shots(adventure_title, character, framing)
            .map(|(number, _)| number)
            .max()
            .map_or(0, |last| last + 1);

        let adventure = slug(adventure_title);
        let name = slug(&character.name);
        let shot = shot(framing);

        format!("{adventure}/{name}/{name}_{shot}_{next}")
    }

    pub fn latest(
        &self,
        adventure_title: &str,
        character: &Character,
        framing: ImageFraming,
    ) -> Option<PathBuf> {
        self.numbered_shots(adventure_title, character, framing)
            .max_by_key(|(number, path)| (*number, path.file_name().map(OsStr::to_owned)))
            .map(|(_, path)| path)
    }

    /// Every shot of one framing already on disk, paired with the number `prefix` gave it.
    fn numbered_shots(
        &self,
        adventure_title: &str,
        character: &Character,
        framing: ImageFraming,
    ) -> impl Iterator<Item = (u32, PathBuf)> {
        let directory = self
            .root
            .join(slug(adventure_title))
            .join(slug(&character.name));
        let head = format!("{}_{}_", slug(&character.name), shot(framing));

        std::fs::read_dir(directory)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
            .filter_map(move |path| {
                let number = shot_number(path.file_stem()?, &head)?;
                Some((number, path))
            })
    }
}

/// The number between the shot name and the `_00001_` counter ComfyUI appends on save.
fn shot_number(stem: &OsStr, head: &str) -> Option<u32> {
    let rest = stem.to_str()?.strip_prefix(head)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();

    digits.parse().ok()
}

fn shot(framing: ImageFraming) -> &'static str {
    match framing {
        ImageFraming::Portrait => "portrait",
        ImageFraming::FullBody => "full_body",
        ImageFraming::FullBodyPose => "pose",
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
