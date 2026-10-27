use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::{from_timestamp_to_str, parse_json_array, to_json_array};
use crate::backend::{
    domain::memory::State,
    persistence::{
        db::{from_str_to_timestamp, from_str_to_uuid},
        error::PersistenceError,
    },
};

struct StateRow {
    id: String,
    adventure_id: String,
    day: String,
    place: String,
    characters_present: String,
    open_threads: String,
    conditions: String,
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

pub async fn get_active_candidates(
    conn: &mut SqliteConnection,
    adventure_id: &str,
) -> Result<Vec<String>, PersistenceError> {
    let row = sqlx::query("SELECT characters_present FROM state WHERE adventure_id = ?1")
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

pub async fn upsert_state(
    conn: &mut SqliteConnection,
    state: &State,
) -> Result<(), PersistenceError> {
    let characters_present =
        to_json_array(&state.characters_present)?.unwrap_or_else(|| "[]".into());
    let open_threads = to_json_array(&state.open_threads)?.unwrap_or_else(|| "[]".into());
    let conditions = to_json_array(&state.conditions)?.unwrap_or_else(|| "[]".into());

    sqlx::query(
        "INSERT INTO state (id, adventure_id, day, place, characters_present,
                            open_threads, conditions, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(adventure_id) DO UPDATE SET
            day                = excluded.day,
            place              = excluded.place,
            characters_present = excluded.characters_present,
            open_threads       = excluded.open_threads,
            conditions         = excluded.conditions,
            updated_at         = excluded.updated_at",
    )
    .bind(state.id.to_string())
    .bind(state.adventure_id.to_string())
    .bind(&state.day)
    .bind(&state.place)
    .bind(characters_present)
    .bind(open_threads)
    .bind(conditions)
    .bind(from_timestamp_to_str(&state.updated_at))
    .execute(&mut *conn)
    .await?;

    Ok(())
}

impl TryFrom<StateRow> for State {
    type Error = PersistenceError;

    fn try_from(row: StateRow) -> Result<Self, Self::Error> {
        Ok(State {
            id: from_str_to_uuid(&row.id)?,
            adventure_id: from_str_to_uuid(&row.adventure_id)?,
            day: row.day,
            place: row.place,
            characters_present: parse_json_array(Some(&row.characters_present))?,
            open_threads: parse_json_array(Some(&row.open_threads))?,
            conditions: parse_json_array(Some(&row.conditions))?,
            updated_at: from_str_to_timestamp(&row.updated_at)?,
        })
    }
}
