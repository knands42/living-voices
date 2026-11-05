use std::{fmt::Display, vec};

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
}

impl Character {
    pub const DEFAULT_PLAYER_NAME: &'static str = "You";

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
        }
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

    /// The visual source an image prompt is built from: the generated identity, or
    /// the author's own description when they chose to keep theirs as-is.
    pub fn image_identity(&self) -> Option<&str> {
        self.appearance_identity_prompt
            .as_deref()
            .or(self.appearance.as_deref())
    }

    pub fn render(&self) -> String {
        let mut parts = vec![format!("{}.", self.name)];
        if !self.headline.trim().is_empty() {
            parts.push(format!("Headline: {}", self.headline.clone()));
        }
        if !self.backstory.trim().is_empty() {
            parts.push(format!("Backstory: {}", self.backstory));
        }
        if !self.traits.is_empty() {
            parts.push(format!("Traits: {}.", self.traits.join(", ")));
        }
        if let Some(appearance) = &self.appearance {
            parts.push(format!("Appearance: {}.", appearance));
        }

        parts.join(" ")
    }
}
