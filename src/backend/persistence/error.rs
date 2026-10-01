use thiserror::Error;

use crate::backend::persistence::seeder::SeedAdventure;

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("could not prepare database directory {path}: {source}")]
    PrepareDirectory {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("`{adventure}` refers to unknown character `{character}`")]
    UnknownCharacter {
        adventure: String,
        character: String,
    },

    #[error("`{adventure}` is not valid: {reason}")]
    InvalidAdventure { adventure: String, reason: String },
}

impl PersistenceError {
    pub fn unknown_seed_character(adventure: &SeedAdventure, character: &str) -> Self {
        Self::UnknownCharacter {
            adventure: adventure.title.to_string(),
            character: character.to_string(),
        }
    }

    pub fn invalid_seed(adventure: &SeedAdventure, reason: impl Into<String>) -> Self {
        Self::InvalidAdventure {
            adventure: adventure.title.to_string(),
            reason: reason.into(),
        }
    }
}
