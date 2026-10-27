use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::{
    domain::memory::{Act, ActStatus},
    persistence::{db::from_str_to_timestamp, db::from_str_to_uuid, error::PersistenceError},
};

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
            WHERE adventure_id = ?1 AND status IN ('resolved', 'failed', 'bypassed')
            ORDER BY ordinal DESC
            LIMIT 1
        )"#,
    )
    .bind(adventure_id.to_string())
    .execute(&mut *conn)
    .await?;

    Ok(())
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
            digest_to_seq: row.digest_to_seq,
            from_seq: row.from_seq,
            to_seq: row.to_seq,
            last_closure_score: row.last_closure_score,
            created_at: from_str_to_timestamp(&row.created_at)?,
            updated_at: from_str_to_timestamp(&row.updated_at)?,
        })
    }
}
