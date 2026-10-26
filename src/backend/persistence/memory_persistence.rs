use sqlx::{QueryBuilder, Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::{from_timestamp_to_str, parse_json_array, to_json_array};
use crate::backend::{
    domain::memory::{Act, ActStatus, Fact, State},
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

struct ActRow {
    id: String,
    adventure_id: String,
    ordinal: u16,
    goal: String,
    trigger: Option<String>,
    status: String,
    index_line: Option<String>,
    digest: Option<String>,
    running_digest: Option<String>,
    digest_to_seq: Option<u16>,
    from_seq: u16,
    to_seq: u16,
    last_closure_score: u16,
    created_at: String,
    updated_at: String,
}

impl ActRow {
    fn decode(row: &sqlx::sqlite::SqliteRow) -> Self {
        Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            ordinal: row.get::<u16, _>("ordinal"),
            goal: row.get("goal"),
            trigger: row.get("trigger"),
            status: row.get("status"),
            index_line: row.get("index_line"),
            digest: row.get("digest"),
            running_digest: row.get("running_digest"),
            digest_to_seq: row.get::<Option<u16>, _>("digest_to_seq"),
            from_seq: row.get::<u16, _>("from_seq"),
            to_seq: row.get::<u16, _>("to_seq"),
            last_closure_score: row.get::<u16, _>("last_closure_score"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// How many messages sit inside the adventure's active act.
pub async fn count_messages_per_act(
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

pub async fn get_act(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
) -> Result<Vec<Act>, PersistenceError> {
    let row = sqlx::query(
        "SELECT id, adventure_id, ordinal, goal, trigger, status, index_line, digest, running_digest, digest_to_seq, from_seq, to_seq, last_closure_score, created_at, updated_at
         FROM act
         WHERE adventure_id = ?1",
    )
        .bind(adventure_id.to_string())
        .fetch_all(&mut *conn)
        .await?;

    row.iter()
        .rev()
        .map(ActRow::decode)
        .map(Act::try_from)
        .collect()
}

pub async fn get_active_act(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
) -> Result<Option<Act>, PersistenceError> {
    let row = sqlx::query(
        "SELECT id, adventure_id, ordinal, goal, trigger, status, index_line, digest,
                running_digest, digest_to_seq, from_seq, to_seq, last_closure_score, created_at, updated_at
         FROM act
         WHERE adventure_id = ?1 AND status = 'active'
         ORDER BY ordinal DESC
         LIMIT 1",
    )
    .bind(adventure_id.to_string())
    .fetch_optional(&mut *conn)
    .await?;

    row.as_ref()
        .map(ActRow::decode)
        .map(Act::try_from)
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

pub async fn insert_facts(
    conn: &mut SqliteConnection,
    facts: &[Fact],
) -> Result<(), PersistenceError> {
    if facts.is_empty() {
        return Ok(());
    }

    let mut qb = QueryBuilder::new(
        "INSERT INTO fact (id, adventure_id, subject, fact, act_id, permanence, created_at) ",
    );

    qb.push_values(facts, |mut row, fact| {
        row.push_bind(fact.id.to_string())
            .push_bind(fact.adventure_id.to_string())
            .push_bind(&fact.subject)
            .push_bind(&fact.fact)
            .push_bind(fact.act_id.to_string())
            .push_bind(fact.permanence.as_str())
            .push_bind(from_timestamp_to_str(&fact.created_at));
    });

    qb.build().execute(&mut *conn).await?;
    Ok(())
}

pub async fn update_act_running_digest(
    conn: &mut SqliteConnection,
    act_id: Uuid,
    running_digest: &str,
    digest_to_seq: u16,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE act
         SET running_digest = ?2,
             digest_to_seq  = ?3
         WHERE id = ?1",
    )
    .bind(act_id.to_string())
    .bind(running_digest)
    .bind(digest_to_seq)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn update_active_act_last_closure_score(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    score: u16,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE act
         SET last_closure_score = ?2
         WHERE adventure_id = ?1 AND status = 'active'",
    )
    .bind(adventure_id.to_string())
    .bind(score)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn insert_act(conn: &mut SqliteConnection, act: &Act) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT INTO act (id, adventure_id, ordinal, goal, trigger, status, from_seq)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(act.id.to_string())
    .bind(act.adventure_id.to_string())
    .bind(act.ordinal)
    .bind(&act.goal)
    .bind(act.trigger.as_deref())
    .bind(act.status.to_string())
    .bind(act.from_seq)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn close_act(
    conn: &mut SqliteConnection,
    act_id: Uuid,
    index_line: &str,
    digest: &str,
    status: &str,
    to_seq: u16,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE act
         SET index_line = ?2,
             digest     = ?3,
             status     = ?4,
             to_seq     = ?5
         WHERE id = ?1",
    )
    .bind(act_id.to_string())
    .bind(index_line)
    .bind(digest)
    .bind(status)
    .bind(to_seq)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn activate_next_act(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
) -> Result<(), PersistenceError> {
    sqlx::query(
        r#"
        UPDATE act
        SET status = 'active'
        WHERE adventure_id = ?1 AND ordinal = (
            SELECT ordinal + 1
            FROM act
            WHERE adventure_id = ?1 AND status = 'closed'
            ORDER BY ordinal DESC
            LIMIT 1
        )"#,
    )
    .bind(adventure_id.to_string())
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

impl TryFrom<ActRow> for Act {
    type Error = PersistenceError;

    fn try_from(row: ActRow) -> Result<Self, Self::Error> {
        let status =
            ActStatus::from_str(&row.status).ok_or_else(|| PersistenceError::InvalidAdventure {
                adventure: row.adventure_id.clone(),
                reason: format!("unknown act status `{}`", row.status),
            })?;

        Ok(Act {
            id: from_str_to_uuid(&row.id)?,
            adventure_id: from_str_to_uuid(&row.adventure_id)?,
            ordinal: row.ordinal,
            goal: row.goal,
            trigger: row.trigger,
            status,
            index_line: row.index_line,
            digest: row.digest,
            running_digest: row.running_digest,
            digest_to_seq: row.digest_to_seq.map(|seq| seq),
            from_seq: row.from_seq,
            to_seq: row.to_seq,
            last_closure_score: row.last_closure_score,
            created_at: from_str_to_timestamp(&row.created_at)?,
            updated_at: from_str_to_timestamp(&row.updated_at)?,
        })
    }
}
