use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::from_str_to_timestamp;
use crate::backend::{
    domain::message::{Message, MessageKind, MessagePart, MessageParts},
    persistence::{db::from_timestamp_to_str, error::PersistenceError},
};

struct MessageRow {
    seq: u64,
    adventure_id: String,
    narrator_id: Option<String>,
    character_id: Option<String>,
    speaker_name: String,
    kind: String,
    text: String,
    target_name: Option<String>,
    secondary_kind: Option<String>,
    secondary_text: Option<String>,
    secondary_target_name: Option<String>,
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
            kind: row.get("kind"),
            text: row.get("text"),
            target_name: row.get("target_name"),
            secondary_kind: row.get("secondary_kind"),
            secondary_text: row.get("secondary_text"),
            secondary_target_name: row.get("secondary_target_name"),
            created_at: row.get("created_at"),
        }
    }
}

/// Append a new message row. `seq` is assigned by `AUTOINCREMENT` and stored
pub async fn insert_message(
    conn: &mut SqliteConnection,
    msg: &Message,
) -> Result<Message, PersistenceError> {
    let primary = msg.parts.primary();
    let secondary = msg.parts.secondary();

    let row = sqlx::query(
        "INSERT INTO message (adventure_id, character_id, speaker_name, kind, text, target_name,
                              secondary_kind, secondary_text, secondary_target_name, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         RETURNING seq",
    )
    .bind(msg.adventure_id.to_string())
    .bind(msg.character_id.map(|id| id.to_string()))
    .bind(&msg.speaker_name)
    .bind(primary.kind.as_str())
    .bind(&primary.text)
    .bind(primary.target_name.as_deref())
    .bind(secondary.map(|p| p.kind.as_str()))
    .bind(secondary.map(|p| p.text.as_str()))
    .bind(secondary.and_then(|p| p.target_name.as_deref()))
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
    after_seq: i64,
) -> Result<Vec<Message>, PersistenceError> {
    let rows = sqlx::query(
        "SELECT seq, adventure_id, character_id, narrator_id, speaker_name, kind, text, target_name,
                secondary_kind, secondary_text, secondary_target_name, created_at
         FROM message
         WHERE adventure_id = ?1 AND seq > ?2
         ORDER BY seq ASC",
    )
    .bind(adventure_id.to_string())
    .bind(after_seq)
    .fetch_all(&mut *conn)
    .await?;

    rows.iter()
        .map(MessageRow::decode)
        .map(Message::try_from)
        .collect()
}

impl TryFrom<MessageRow> for Message {
    type Error = PersistenceError;

    fn try_from(row: MessageRow) -> Result<Self, Self::Error> {
        let adventure_id = Uuid::parse_str(&row.adventure_id)
            .map_err(|e| PersistenceError::Database(sqlx::Error::Decode(Box::new(e))))?;
        let created_at = from_str_to_timestamp(&row.created_at)?;

        let character_id = row
            .character_id
            .map(|s| {
                Uuid::parse_str(&s)
                    .map_err(|e| PersistenceError::Database(sqlx::Error::Decode(Box::new(e))))
            })
            .transpose()?;

        let narrator_id = row
            .narrator_id
            .map(|n| {
                Uuid::parse_str(&n)
                    .map_err(|e| PersistenceError::Database(sqlx::Error::Decode(Box::new(e))))
            })
            .transpose()?;

        let primary = decode_part(&row.kind, row.text, row.target_name)?;
        let parts = match (row.secondary_kind, row.secondary_text) {
            (None, None) => MessageParts::one(primary),
            (Some(kind), Some(text)) => MessageParts::two(
                primary,
                decode_part(&kind, text, row.secondary_target_name)?,
            ),
            // The DB CHECK guarantees kind and text arrive together; anything else is corruption
            _ => {
                return Err(PersistenceError::Database(sqlx::Error::Decode(
                    "secondary part is missing its kind or text".into(),
                )));
            }
        };

        Ok(Message {
            seq: Some(row.seq),
            adventure_id,
            character_id,
            narrator_id,
            speaker_name: row.speaker_name,
            parts,
            created_at,
        })
    }
}

fn decode_part(
    kind: &str,
    text: String,
    target_name: Option<String>,
) -> Result<MessagePart, PersistenceError> {
    let kind = MessageKind::from_str(kind).ok_or_else(|| {
        PersistenceError::Database(sqlx::Error::Decode(
            format!("unknown message kind: {kind}").into(),
        ))
    })?;
    Ok(match target_name {
        Some(target) => MessagePart::new(kind, text).with_target(target),
        None => MessagePart::new(kind, text),
    })
}
