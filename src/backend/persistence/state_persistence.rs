use sqlx::sqlite::SqliteRow;
use sqlx::{Error, FromRow, Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::{from_timestamp_to_str, parse_json_array, to_json_array};
use crate::backend::{
    domain::memory::{CharacterCondition, State},
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
    character_conditions: String,
    updated_at: String,
}

impl FromRow<'_, SqliteRow> for StateRow {
    fn from_row(row: &SqliteRow) -> Result<StateRow, Error> {
        Ok(Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            day: row.get("day"),
            place: row.get("place"),
            characters_present: row.get("characters_present"),
            open_threads: row.get("open_threads"),
            character_conditions: row.get("character_conditions"),
            updated_at: row.get("updated_at"),
        })
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
    let row = sqlx::query_as::<_, StateRow>(
        "SELECT id, adventure_id, day, place, characters_present, open_threads,
                character_conditions, updated_at
         FROM state
         WHERE adventure_id = ?1",
    )
    .bind(adventure_id.to_string())
    .fetch_optional(&mut *conn)
    .await?;

    row.map(State::try_from).transpose()
}

pub async fn insert_default_state(
    conn: &mut SqliteConnection,
    id: Uuid,
    adventure_id: Uuid,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT INTO state (id, adventure_id) VALUES (?1, ?2)
         ON CONFLICT(adventure_id) DO NOTHING",
    )
    .bind(id.to_string())
    .bind(adventure_id.to_string())
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn upsert_state(
    conn: &mut SqliteConnection,
    state: &State,
) -> Result<(), PersistenceError> {
    let characters_present =
        to_json_array(&state.characters_present)?.unwrap_or_else(|| "[]".into());
    let open_threads = to_json_array(&state.open_threads)?.unwrap_or_else(|| "[]".into());
    let character_conditions = serde_json::to_string(&state.character_conditions)?;

    sqlx::query(
        "INSERT INTO state (id, adventure_id, day, place, characters_present,
                            open_threads, character_conditions, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(adventure_id) DO UPDATE SET
            day                  = excluded.day,
            place                = excluded.place,
            characters_present   = excluded.characters_present,
            open_threads         = excluded.open_threads,
            character_conditions = excluded.character_conditions,
            updated_at           = excluded.updated_at",
    )
    .bind(state.id.to_string())
    .bind(state.adventure_id.to_string())
    .bind(&state.day)
    .bind(&state.place)
    .bind(characters_present)
    .bind(open_threads)
    .bind(character_conditions)
    .bind(from_timestamp_to_str(&state.updated_at))
    .execute(&mut *conn)
    .await?;

    Ok(())
}

fn parse_character_conditions(text: &str) -> Result<Vec<CharacterCondition>, PersistenceError> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(text).map_err(PersistenceError::ParsingError)
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
            character_conditions: parse_character_conditions(&row.character_conditions)?,
            updated_at: from_str_to_timestamp(&row.updated_at)?,
        })
    }
}
