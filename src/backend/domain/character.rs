use std::fmt::Display;

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CharacterStatus {
    #[default]
    Present,
    Away,
    Unavailable,
}

impl CharacterStatus {
    pub const ALL: [CharacterStatus; 3] = [
        CharacterStatus::Present,
        CharacterStatus::Away,
        CharacterStatus::Unavailable,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            CharacterStatus::Away => "away",
            CharacterStatus::Present => "present",
            CharacterStatus::Unavailable => "unavailable",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "away" => CharacterStatus::Away,
            "present" => CharacterStatus::Present,
            "unavailable" => CharacterStatus::Unavailable,
            _ => CharacterStatus::Present,
        }
    }
}

impl From<String> for CharacterStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "away" => CharacterStatus::Away,
            "present" => CharacterStatus::Present,
            "unavailable" => CharacterStatus::Unavailable,
            _ => CharacterStatus::Present,
        }
    }
}

impl Display for CharacterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How much of the character an image should show. None of the image prompt slots
/// know about framing: it is applied when the finished prompt is assembled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFraming {
    Portrait,
    FullBody,
}

#[derive(Clone, Debug)]
pub struct Character {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub name: String,
    pub headline: String,
    pub status: CharacterStatus,
    pub backstory: String,
    /// Used for a "poetic" description of the character
    pub appearance: Option<String>,
    /// Face, hair, build and marks: who the character is, without clothing or pose.
    pub appearance_identity_prompt: Option<String>,
    /// What the character wears right now, kept apart from identity so it can change.
    pub appearance_current_outfit: Option<String>,
    /// The pose and action of the current moment.
    pub appearance_current_pose: Option<String>,
    pub portrait_path: Option<String>,
    pub is_player: bool,
    pub traits: Vec<String>,    // Speaks only in tidal registers, ...
    pub tags: Vec<String>,      // Protagonist, Brave ...
    pub relations: Vec<String>, // Son of Dragoon
    pub message_example: Option<String>,
}

impl Character {
    pub const DEFAULT_PLAYER_NAME: &'static str = "You";
    /// The name a character carries from the moment its page opens until the
    /// player types a real one. A character still wearing it is not a cast member.
    pub const PLACEHOLDER_NAME: &'static str = "New character";

    pub fn new(
        id: Uuid,
        name: String,
        adventure_id: Uuid,
        headline: impl Into<String>,
        backstory: impl Into<String>,
        status: CharacterStatus,
        is_player: bool,
    ) -> Self {
        Self {
            id,
            adventure_id,
            name,
            headline: headline.into(),
            backstory: backstory.into(),
            appearance: None,
            appearance_identity_prompt: None,
            appearance_current_outfit: None,
            appearance_current_pose: None,
            status,
            portrait_path: None,
            is_player,
            traits: vec![],
            tags: vec![],
            relations: vec![],
            message_example: None,
        }
    }

    /// The character a fresh page edits. It is persisted immediately so every field
    /// can be saved on its own, but it carries nothing yet — the name is a
    /// placeholder the player is expected to replace.
    pub fn draft(adventure_id: Uuid) -> Self {
        Self::new(
            Uuid::new_v4(),
            Self::PLACEHOLDER_NAME.to_string(),
            adventure_id,
            "",
            "",
            CharacterStatus::Present,
            false,
        )
    }

    /// True while the character is still the placeholder its page opened with.
    pub fn is_placeholder(&self) -> bool {
        self.name == Self::PLACEHOLDER_NAME
    }

    pub fn with_portrait(mut self, portrait_path: impl Into<String>) -> Self {
        self.portrait_path = Some(portrait_path.into());
        self
    }

    pub fn with_appearance(mut self, appearance: impl Into<String>) -> Self {
        self.appearance = Some(appearance.into());
        self
    }

    pub fn with_appearance_identity_prompt(
        mut self,
        appearance_identity_prompt: impl Into<String>,
    ) -> Self {
        self.appearance_identity_prompt = Some(appearance_identity_prompt.into());
        self
    }

    pub fn with_appearance_current_outfit(
        mut self,
        appearance_current_outfit: impl Into<String>,
    ) -> Self {
        self.appearance_current_outfit = Some(appearance_current_outfit.into());
        self
    }

    pub fn with_appearance_current_pose(
        mut self,
        appearance_current_pose: impl Into<String>,
    ) -> Self {
        self.appearance_current_pose = Some(appearance_current_pose.into());
        self
    }

    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }

    pub fn add_trait(&mut self, trait_: String) {
        self.traits.push(trait_);
    }

    pub fn remove_tag(&mut self, tag: String) {
        self.tags.retain(|t| t != &tag);
    }

    pub fn add_relation(&mut self, relation: String) {
        self.relations.push(relation);
    }

    pub fn with_message_example(mut self, message_example: impl Into<String>) -> Self {
        self.message_example = Some(message_example.into());
        self
    }

    /// The visual source an image prompt is built from: the generated identity, or
    /// the author's own description when they chose to keep theirs as-is.
    pub fn image_identity(&self) -> Option<&str> {
        self.appearance_identity_prompt
            .as_deref()
            .or(self.appearance.as_deref())
    }

    pub fn render(&self) -> String {
        let mut lines = vec![format!("### {}", self.name)];
        if !self.headline.trim().is_empty() {
            lines.push(format!(
                "*Headline*: {}.",
                self.headline.trim_end_matches(".")
            ));
        }
        if !self.backstory.trim().is_empty() {
            lines.push(format!(
                "*Backstory*: {}.",
                self.backstory.trim_end_matches(".")
            ));
        }
        if let Some(section) = render_list("*Traits*", &self.traits) {
            lines.push(section);
        }
        if let Some(section) = render_list("*Relations*", &self.relations) {
            lines.push(section);
        }
        if let Some(appearance) = &self.appearance {
            lines.push(format!(
                "*Appearance*: {}.",
                appearance.trim_end_matches(".")
            ));
        }

        lines.join("\n")
    }

    pub fn message_example(&self) -> Option<&str> {
        let example = self.message_example.as_deref()?.trim();

        if example.is_empty() {
            return None;
        }

        Some(example)
    }
}

fn render_list(label: &str, items: &[String]) -> Option<String> {
    let kept: Vec<&str> = items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect();

    if kept.is_empty() {
        return None;
    }

    Some(format!(
        "{label}: {}.",
        kept.join(", ").trim_end_matches(".")
    ))
}
