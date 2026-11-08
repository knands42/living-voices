use crate::backend::domain::adventure::{AdventureStatus, AdventureSummary};
use crate::backend::domain::character::{Character, CharacterStatus};
use crate::backend::domain::lore::Lore;
use crate::backend::domain::narrator::Narrator;
use crate::backend::persistence::db::{from_str_to_timestamp, from_str_to_uuid, parse_json_array};
use crate::backend::{
    domain::adventure::Adventure,
    persistence::{
        error::PersistenceError,
        tags::{ensure_tag, normalize},
    },
};
use sqlx::sqlite::SqliteRow;
use sqlx::{Error, FromRow, Row, SqliteConnection};
use uuid::Uuid;

// TODO: Let sqlx parse & implement the trait FromRow using #[sqlx(FromRow]
struct AdventureRow {
    id: String,
    title: String,
    headline: String,
    tone: String,
    status: String,
    created_at: String,
    updated_at: String,
}

impl FromRow<'_, SqliteRow> for AdventureRow {
    fn from_row(row: &SqliteRow) -> Result<AdventureRow, sqlx::Error> {
        Ok(Self {
            id: row.get("id"),
            title: row.get("title"),
            headline: row.get("headline"),
            tone: row.get("tone"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

impl TryFrom<AdventureRow> for AdventureSummary {
    type Error = PersistenceError;
    fn try_from(value: AdventureRow) -> Result<Self, Self::Error> {
        let parsed_id = from_str_to_uuid(&value.id)?;
        let parsed_created_at = from_str_to_timestamp(&value.created_at)?;
        let parsed_updated_at = from_str_to_timestamp(&value.updated_at)?;

        Ok(AdventureSummary::new(
            parsed_id,
            value.title,
            value.headline,
            value.tone,
            AdventureStatus::from_str(&value.status),
            parsed_created_at,
            parsed_updated_at,
        ))
    }
}

struct CompleteAdventureRow {
    adventure_id: String,
    adventure_title: String,
    adventure_headline: String,
    adventure_tone: String,
    adventure_status: String,
    adventure_created_at: String,
    adventure_updated_at: String,
    adventure_tags: String,
    adventure_lore: String,
    lore_id: String,
    character_id: String,
    character_name: String,
    character_genre: String,
    character_description: String,
    character_backstory: String,
    character_appearance: Option<String>,
    character_appearance_current_body: Option<String>,
    character_appearance_current_outfit: String,
    character_appearance_current_pose: Option<String>,
    character_status: String,
    character_is_player: bool,
    character_tags: String,
    character_traits: String,
    character_relations: String,
    character_message_example: Option<String>,
    narrator_id: String,
    narrator_name: String,
    narrator_tone: String,
}

impl FromRow<'_, SqliteRow> for CompleteAdventureRow {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            adventure_id: row.get("adventure_id"),
            adventure_title: row.get("adventure_title"),
            adventure_headline: row.get("adventure_headline"),
            adventure_tone: row.get("adventure_tone"),
            adventure_status: row.get("adventure_status"),
            adventure_created_at: row.get("adventure_created_at"),
            adventure_updated_at: row.get("adventure_updated_at"),
            adventure_tags: row.get("adventure_tags"),
            adventure_lore: row.get("adventure_lore"),
            lore_id: row.get("lore_id"),
            character_id: row.get("character_id"),
            character_name: row.get("character_name"),
            character_genre: row.get("character_genre"),
            character_description: row.get("character_description"),
            character_backstory: row.get("character_backstory"),
            character_appearance: row.get("character_appearance"),
            character_appearance_current_body: row.get("character_appearance_current_body"),
            character_appearance_current_outfit: row.get("character_appearance_current_outfit"),
            character_appearance_current_pose: row.get("character_appearance_current_pose"),
            character_status: row.get("character_status"),
            character_is_player: row.get::<bool, _>("character_is_player"),
            character_tags: row.get("character_tags"),
            character_traits: row.get("character_traits"),
            character_relations: row.get("character_relations"),
            character_message_example: row.get("character_message_example"),
            narrator_id: row.get("narrator_id"),
            narrator_name: row.get("narrator_name"),
            narrator_tone: row.get("narrator_tone"),
        })
    }
}

fn character_from_row(value: &CompleteAdventureRow) -> Result<Character, PersistenceError> {
    let character_id_uuid = from_str_to_uuid(value.character_id.as_str())?;
    let adventure_id_uuid = from_str_to_uuid(value.adventure_id.as_str())?;
    let character_tags = parse_json_array(Some(value.character_tags.as_str()))?;
    let character_traits = parse_json_array(Some(value.character_traits.as_str()))?;
    let character_relations = parse_json_array(Some(value.character_relations.as_str()))?;

    let mut character = Character::new(
        character_id_uuid,
        value.character_name.clone(),
        value.character_genre.clone(),
        adventure_id_uuid,
        value.character_description.clone(),
        value.character_backstory.clone(),
        CharacterStatus::from(value.character_status.clone()),
        value.character_is_player,
    );
    character.appearance = value.character_appearance.clone();
    character.appearance_current_body = value.character_appearance_current_body.clone();
    character.appearance_current_outfit = value.character_appearance_current_outfit.clone();
    character.appearance_current_pose = value.character_appearance_current_pose.clone();
    character.message_example = value.character_message_example.clone();

    character_tags
        .iter()
        .for_each(|tag| character.add_tag(tag.clone()));
    character_traits
        .iter()
        .for_each(|trait_| character.add_trait(trait_.clone()));
    character_relations
        .iter()
        .for_each(|relation| character.add_relation(relation.clone()));

    Ok(character)
}

impl TryFrom<CompleteAdventureRow> for Adventure {
    type Error = PersistenceError;
    fn try_from(value: CompleteAdventureRow) -> Result<Self, Self::Error> {
        let adventure_id_uuid = from_str_to_uuid(value.adventure_id.as_str())?;
        let narrator_id_uuid = from_str_to_uuid(value.narrator_id.as_str())?;
        let lore_id_uuid = from_str_to_uuid(value.lore_id.as_str())?;
        let adventure_created_at_utc = from_str_to_timestamp(value.adventure_created_at.as_str())?;
        let adventure_updated_at_utc = from_str_to_timestamp(value.adventure_updated_at.as_str())?;

        let character = character_from_row(&value)?;

        let narrator = Narrator::new(
            narrator_id_uuid,
            adventure_id_uuid,
            value.narrator_name.clone(),
            value.narrator_tone.clone(),
        );

        let lore_texts = parse_json_array(Some(value.adventure_lore.as_str()))?;
        let lore = Lore::new(lore_id_uuid, lore_texts.join("\n"));

        let mut new_adventure = Adventure::new(
            adventure_id_uuid,
            value.adventure_title.clone(),
            value.adventure_headline.clone(),
            narrator,
            value.adventure_tone.clone(),
            lore,
            AdventureStatus::from_str(&value.adventure_status),
            adventure_created_at_utc,
            adventure_updated_at_utc,
        );

        let adventure_tags = parse_json_array(Some(value.adventure_tags.as_str()))?;

        new_adventure.add_character(character);
        new_adventure.add_tags(adventure_tags);

        Ok(new_adventure)
    }
}

pub async fn insert_adventure(
    conn: &mut SqliteConnection,
    id: Uuid,
    title: &str,
    headline: &str,
    tone: &str,
    tags: Vec<String>,
) -> Result<AdventureSummary, PersistenceError> {
    let adventure_row: AdventureRow = sqlx::query_as(
        "INSERT INTO adventure (id, title, headline, tone)
         VALUES (?1, ?2, ?3, ?4)
         RETURNING *",
    )
    .bind(id.to_string())
    .bind(title)
    .bind(headline)
    .bind(tone)
    .fetch_one(&mut *conn)
    .await?;

    for tag in &tags {
        ensure_tag(conn, tag).await?;
        attach_tag(conn, id, tag).await?;
    }

    AdventureSummary::try_from(adventure_row)
}

pub async fn get_adventure(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
) -> Result<Adventure, PersistenceError> {
    let rows = sqlx::query_as::<_, CompleteAdventureRow>(
        r#"
SELECT a.id                      AS adventure_id,
       a.title                   AS adventure_title,
       a.headline             AS adventure_headline,
       a.tone                   AS adventure_tone,
       a.status                 AS adventure_status,
       a.created_at             AS adventure_created_at,
       a.updated_at             AS adventure_updated_at,
       COALESCE(at.tags, '[]')   AS adventure_tags,
       COALESCE(l.lore, '[]')    AS adventure_lore,
       l.lore_id                  AS lore_id,
       c.id                      AS character_id,
       c.name                   AS character_name,
       c.genre                  AS character_genre,
       c.headline                AS character_description,
       c.backstory                AS character_backstory,
       c.appearance               AS character_appearance,
       c.appearance_current_body AS character_appearance_current_body,
       c.appearance_current_outfit AS character_appearance_current_outfit,
       c.appearance_current_pose AS character_appearance_current_pose,
       c.status                   AS character_status,
       c.is_player AS character_is_player,
       COALESCE(ctag.tags, '[]') AS character_tags,
       COALESCE(ct.trait, '[]')  AS character_traits,
       c.relations AS character_relations,
       c.message_example AS character_message_example,
       n.id AS narrator_id, n.name AS narrator_name, n.tone AS narrator_tone
FROM adventure AS a
         LEFT JOIN (SELECT adventure_id,
                           '[' || GROUP_CONCAT('"' || tag || '"', ',') || ']' AS tags
                    FROM adventure_tag GROUP BY adventure_id) AS at
                   ON at.adventure_id = a.id
         LEFT JOIN (SELECT adventure_id,
                           '[' || GROUP_CONCAT('"' || text || '"', ',') || ']' AS lore,
                           MIN(id) AS lore_id,
                           '' AS lore_text
                    FROM lore GROUP BY adventure_id) AS l
                   ON l.adventure_id = a.id
         LEFT JOIN character AS c ON c.adventure_id = a.id
         LEFT JOIN (SELECT character_id,
                           '[' || GROUP_CONCAT('"' || tag || '"', ',') || ']' AS tags
                    FROM character_tag GROUP BY character_id) AS ctag
                   ON ctag.character_id = c.id
         LEFT JOIN (SELECT character_id,
                           '[' || GROUP_CONCAT('"' || value || '"', ',') || ']' AS trait
                    FROM character_trait GROUP BY character_id) AS ct
                   ON ct.character_id = c.id
         LEFT JOIN narrator AS n ON n.adventure_id = a.id
WHERE a.id = ?
ORDER BY a.updated_at DESC, c.name DESC;
"#,
    )
    .bind(adventure_id.to_string())
    .fetch_all(&mut *conn)
    .await?;

    let mut rows = rows.into_iter();
    let first = rows.next().ok_or(sqlx::Error::RowNotFound)?;

    let mut adventure = Adventure::try_from(first)?;
    for row in rows {
        adventure.add_character(character_from_row(&row)?);
    }

    Ok(adventure)
}

pub async fn get_adventure_summaries(
    conn: &mut SqliteConnection,
) -> Result<Vec<AdventureSummary>, PersistenceError> {
    let rows = sqlx::query_as::<_, AdventureRow>("SELECT * FROM adventure LIMIT 10")
        .fetch_all(&mut *conn)
        .await?;

    let mut adventures = Vec::with_capacity(10);
    for row in rows {
        adventures.push(AdventureSummary::try_from(row)?)
    }

    Ok(adventures)
}

pub async fn attach_tag(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    tag: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR IGNORE INTO adventure_tag (adventure_id, tag) VALUES (?1, ?2)")
        .bind(adventure_id.to_string())
        .bind(normalize(tag))
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn update_adventure_status(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    status: AdventureStatus,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE adventure SET status = ?1 WHERE id = ?2")
        .bind(status.as_str())
        .bind(adventure_id.to_string())
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn update_adventure(
    conn: &mut SqliteConnection,
    adventure: &Adventure,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE adventure SET title = ?1, headline = ?2, tone = ?3, status = ?4 WHERE id = ?5",
    )
    .bind(&adventure.title)
    .bind(&adventure.headline)
    .bind(&adventure.tone)
    .bind(adventure.status.as_str())
    .bind(adventure.id.to_string())
    .execute(&mut *conn)
    .await?;

    // TODO: This will move out when narrator manager exists
    sqlx::query("UPDATE narrator SET name = ?1, tone = ?2 WHERE adventure_id = ?3")
        .bind(&adventure.narrator.name)
        .bind(&adventure.narrator.tone)
        .bind(adventure.id.to_string())
        .execute(&mut *conn)
        .await?;

    sqlx::query("UPDATE lore SET text = ?1 WHERE adventure_id = ?2")
        .bind(&adventure.lore.text)
        .bind(adventure.id.to_string())
        .execute(&mut *conn)
        .await?;

    Ok(())
}
