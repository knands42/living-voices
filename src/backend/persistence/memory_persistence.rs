use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::parse_json_array;
use crate::backend::{
    domain::memory::{Act, State},
    persistence::{
        db::{from_str_to_timestamp, from_str_to_uuid},
        error::PersistenceError,
    },
};

struct StateRow {
    id: String,
    adventure_id: String,
    day: Option<String>,
    place: Option<String>,
    characters_present: Option<String>,
    open_threads: Option<String>,
    conditions: Option<String>,
    updated_at: String,
}

impl StateRow {
    fn decode(row: &sqlx::sqlite::SqliteRow) -> Self {
        Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            day: row.get("day"),
            place: row.get("place"),
            characters_present: row.get("characters_present"),
            open_threads: row.get("open_threads"),
            conditions: row.get("conditions"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// How many messages sit inside the adventure's active act.
pub async fn count_uncovered_per_act(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
) -> Result<u16, PersistenceError> {
    let adventure_id = adventure_id.to_string();

    let row = sqlx::query(
        "SELECT COUNT(*) AS count
         FROM message
         WHERE adventure_id = ?1
           AND seq >= COALESCE(
               (SELECT from_seq FROM act WHERE adventure_id = ?1 AND status = 'active'),
               0
           )",
    )
    .bind(&adventure_id)
    .fetch_one(&mut *conn)
    .await?;

    Ok(row.get::<u16, _>("count"))
}

pub async fn get_active_candidates(
    conn: &mut SqliteConnection,
    adventure_id: &str,
) -> Result<Vec<String>, PersistenceError> {
    let row = sqlx::query("SELECT characters_present AS c FROM state WHERE adventure_id = ?1")
        .bind(adventure_id)
        .fetch_one(&mut *conn)
        .await?;

    parse_json_array(row.get("characters_present"))
}

/// The single live `state` row for an adventure, if one exists yet.
pub async fn get_state(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
) -> Result<Option<State>, PersistenceError> {
    let row = sqlx::query(
        "SELECT id, adventure_id, day, place, characters_present, open_threads,
                conditions, updated_at
         FROM state
         WHERE adventure_id = ?1",
    )
    .bind(adventure_id.to_string())
    .fetch_optional(&mut *conn)
    .await?;

    row.as_ref()
        .map(StateRow::decode)
        .map(State::try_from)
        .transpose()
}

impl TryFrom<StateRow> for State {
    type Error = PersistenceError;

    fn try_from(row: StateRow) -> Result<Self, Self::Error> {
        Ok(State {
            id: from_str_to_uuid(&row.id)?,
            adventure_id: from_str_to_uuid(&row.adventure_id)?,
            day: row.day,
            place: row.place,
            characters_present: parse_json_array(row.characters_present.as_deref())?,
            open_threads: parse_json_array(row.open_threads.as_deref())?,
            conditions: parse_json_array(row.conditions.as_deref())?,
            updated_at: from_str_to_timestamp(&row.updated_at)?,
        })
    }
}
