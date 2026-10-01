use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::backend::persistence::error::PersistenceError;

const MAX_CONNECTIONS: u32 = 4;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn open_pool(db_path: &str) -> Result<SqlitePool, PersistenceError> {
    ensure_parent_dir(db_path)?;

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(BUSY_TIMEOUT);

    let pool = SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect_with(options)
        .await?;

    Ok(pool)
}

fn ensure_parent_dir(db_path: &str) -> Result<(), PersistenceError> {
    let path = Path::new(db_path);
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    if parent.as_os_str().is_empty() || parent.is_dir() {
        return Ok(());
    }

    std::fs::create_dir_all(parent).map_err(|source| PersistenceError::PrepareDirectory {
        path: parent.display().to_string(),
        source,
    })
}

pub fn timestamp(at: &DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}
