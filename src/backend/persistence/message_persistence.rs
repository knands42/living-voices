use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::{from_str_to_timestamp, from_str_to_uuid};
use crate::backend::{
    domain::message::Message,
    persistence::{db::from_timestamp_to_str, error::PersistenceError},
};

struct MessageRow {
    seq: u64,
    adventure_id: String,
    narrator_id: Option<String>,
    character_id: Option<String>,
    speaker_name: String,
    text: String,
    ooc: bool,
    created_at: String,
}

impl MessageRow {
    fn decode(row: &sqlx::sqlite::SqliteRow) -> Self {
        Self {
            seq: row.get("seq"),
            adventure_id: row.get("adventure_id"),
            narrator_id: row.get("narrator_id"),
            character_id: row.get("character_id"),
            speaker_name: row.get("speaker_name"),
            text: row.get("text"),
            ooc: row.get::<u8, _>("ooc") != 0,
            created_at: row.get("created_at"),
        }
    }
}

/// Append a new message row. `seq` is assigned by `AUTOINCREMENT` and stored
pub async fn insert_message(
    conn: &mut SqliteConnection,
    msg: &Message,
) -> Result<Message, PersistenceError> {
    let targeted_ids = encode_targets(msg);

    let row = sqlx::query(
        "INSERT INTO message (adventure_id, character_id, narrator_id, speaker_name, text,
                              targeted_characters, ooc, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         RETURNING seq",
    )
    .bind(msg.adventure_id.to_string())
    .bind(msg.character_id.map(|id| id.to_string()))
    .bind(msg.narrator_id.map(|id| id.to_string()))
    .bind(&msg.speaker_name)
    .bind(&msg.text)
    .bind(targeted_ids)
    .bind(msg.ooc as i64)
    .bind(from_timestamp_to_str(&msg.created_at))
    .fetch_one(&mut *conn)
    .await?;

    let seq: i64 = row.get("seq");
    Ok(Message {
        seq: Some(seq as u64),
        ..msg.clone()
    })
}

pub async fn fetch_since(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    after_seq: u64,
) -> Result<Vec<Message>, PersistenceError> {
    let rows = sqlx::query(
        "SELECT seq, adventure_id, character_id, narrator_id, speaker_name, text, ooc, created_at
         FROM message
         WHERE adventure_id = ?1 AND seq > ?2
         ORDER BY seq ASC",
    )
    .bind(adventure_id.to_string())
    .bind(after_seq as i64)
    .fetch_all(&mut *conn)
    .await?;

    rows.iter()
        .map(MessageRow::decode)
        .map(Message::try_from)
        .collect()
}

pub async fn fetch_last_n(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    limit: i64,
) -> Result<Vec<Message>, PersistenceError> {
    let rows = sqlx::query(
        "SELECT seq, adventure_id, character_id, narrator_id, speaker_name, text, ooc, created_at
         FROM message
         WHERE adventure_id = ?1
         ORDER BY seq DESC
         LIMIT ?2",
    )
    .bind(adventure_id.to_string())
    .bind(limit)
    .fetch_all(&mut *conn)
    .await?;

    rows.iter()
        .rev()
        .map(MessageRow::decode)
        .map(Message::try_from)
        .collect()
}

/// How many messages have been appended since the narrator last spoke
pub async fn count_since_last_narration(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
) -> Result<u8, PersistenceError> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS c FROM message
         WHERE adventure_id = ?1
           AND seq > COALESCE(
               (SELECT MAX(seq) FROM message
                WHERE adventure_id = ?1 AND narrator_id IS NOT NULL),
               0)",
    )
    .bind(adventure_id.to_string())
    .fetch_one(&mut *conn)
    .await?;

    Ok(row.get::<u8, _>("c"))
}

/// The log stores targets as their character ids; whole Character rows live in
/// the character table, so reads do not rehydrate them here.
fn encode_targets(msg: &Message) -> Option<String> {
    let targets = msg.targeted_characters.as_ref()?;
    let ids: Vec<String> = targets.iter().map(|c| c.id.to_string()).collect();
    serde_json::to_string(&ids).ok()
}

impl TryFrom<MessageRow> for Message {
    type Error = PersistenceError;

    fn try_from(row: MessageRow) -> Result<Self, Self::Error> {
        let adventure_id = from_str_to_uuid(&row.adventure_id)?;
        let created_at = from_str_to_timestamp(&row.created_at)?;
        let character_id = row
            .character_id
            .as_deref()
            .map(from_str_to_uuid)
            .transpose()?;
        let narrator_id = row
            .narrator_id
            .as_deref()
            .map(from_str_to_uuid)
            .transpose()?;

        Ok(Message {
            seq: Some(row.seq),
            adventure_id,
            character_id,
            narrator_id,
            speaker_name: row.speaker_name,
            text: row.text,
            targeted_characters: None,
            ooc: row.ooc,
            created_at,
        })
    }
}
