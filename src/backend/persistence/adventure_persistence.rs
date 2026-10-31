use crate::backend::domain::adventure::AdventureSummary;
use crate::backend::domain::character::{Character, CharacterStatus};
use crate::backend::domain::lore::Lore;
use crate::backend::domain::narrator::Narrator;
use crate::backend::persistence::db::{from_str_to_timestamp, from_str_to_uuid, parse_json_array};
use crate::backend::{
    domain::adventure::Adventure,
    persistence::{
        db::from_timestamp_to_str,
        error::PersistenceError,
        tags::{ensure_tag, normalize},
    },
};
use chrono::Utc;
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, Sqlite, SqliteConnection};
use std::collections::HashMap;
use uuid::Uuid;

struct AdventureRow {
    id: String,
    title: String,
    headline: String,
    tone: String,
    last_opened_at: String,
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
            last_opened_at: row.get("last_opened_at"),
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
            parsed_created_at,
            parsed_updated_at,
        ))
    }
}

struct CompleteAdventureRow {
    adventure_id: String,
    adventure_title: String,
    adventure_headline: String,
    adventure_genre: String,
    adventure_tone: String,
    adventure_last_opened_at: String,
    adventure_created_at: String,
    adventure_updated_at: String,
    adventure_tags: String,
    adventure_lore: String,
    lore_id: String,
    lore_text: String,
    character_id: String,
    character_name: String,
    character_description: String,
    character_backstory: String,
    character_status: String,
    character_portrait_path: String,
    character_is_player: bool,
    character_created_at: String,
    character_updated_at: String,
    character_tags: String,
    character_traits: String,
    character_relations: String,
    narrator_id: String,
    narrator_name: String,
    narrator_tone: String,
}

impl CompleteAdventureRow {
    fn decode(row: &SqliteRow) -> Self {
        Self {
            adventure_id: row.get("id"),
            adventure_title: row.get("title"),
            adventure_headline: row.get("description"),
            adventure_genre: row.get("genre"),
            adventure_tone: row.get("tone"),
            adventure_last_opened_at: row.get("last_opened_at"),
            adventure_created_at: row.get("created_at"),
            adventure_updated_at: row.get("updated_at"),
            adventure_tags: row.get("adventure_tags"),
            adventure_lore: row.get("adventure_lore"),
            lore_id: row.get("lore_id"),
            lore_text: row.get("lore_text"),
            character_id: row.get("character_id"),
            character_name: row.get("character_name"),
            character_description: row.get("character_description"),
            character_backstory: row.get("character_backstory"),
            character_status: row.get("character_status"),
            character_portrait_path: row.get("character_portrait_path"),
            character_is_player: row.get::<bool, _>("character_is_player"),
            character_created_at: row.get("character_created_at"),
            character_updated_at: row.get("character_updated_at"),
            character_tags: row.get("character_tags"),
            character_traits: row.get("character_traits"),
            character_relations: row.get("character_relations"),
            narrator_id: row.get("narrator_id"),
            narrator_name: row.get("narrator_name"),
            narrator_tone: row.get("narrator_tone"),
        }
    }

    fn to_adventure(
        adventure_row: Vec<CompleteAdventureRow>,
    ) -> Result<Vec<Adventure>, PersistenceError> {
        let mut new_adventures = HashMap::new();
        for adventure in adventure_row {
            let adventure_id_uuid = from_str_to_uuid(adventure.adventure_id.as_str())?;
            let character_id_uuid = from_str_to_uuid(adventure.character_id.as_str())?;
            let narrator_id_uuid = from_str_to_uuid(adventure.narrator_id.as_str())?;
            let lore_id_uuid = from_str_to_uuid(adventure.lore_id.as_str())?;
            let adventure_created_at_utc =
                from_str_to_timestamp(adventure.adventure_created_at.as_str())?;
            let adventure_updated_at_utc =
                from_str_to_timestamp(adventure.adventure_updated_at.as_str())?;
            let character_tags = parse_json_array(Some(adventure.character_tags.as_str()))?;
            let character_traits = parse_json_array(Some(adventure.character_traits.as_str()))?;
            let character_relations =
                parse_json_array(Some(adventure.character_relations.as_str()))?;

            let mut character = Character::new(
                character_id_uuid,
                adventure.character_name,
                adventure_id_uuid,
                adventure.character_description,
                adventure.character_backstory,
                CharacterStatus::from(adventure.character_status),
                adventure.character_is_player,
            )
            .with_portrait(adventure.character_portrait_path);

            character_tags.iter().for_each(|tag| {
                character.add_tag(tag.clone());
            });
            character_traits.iter().for_each(|trait_| {
                character.add_trait(trait_.clone());
            });
            character_relations.iter().for_each(|relation| {
                character.add_relation(relation.clone());
            });

            let narrator = Narrator::new(
                narrator_id_uuid,
                adventure.narrator_name,
                adventure.narrator_tone,
            );

            let lore_texts = parse_json_array(Some(adventure.adventure_lore.as_str()))?;
            let lore = Lore::new(lore_id_uuid, lore_texts.join("\n"));

            let mut new_adventure = Adventure::new(
                adventure_id_uuid,
                adventure.adventure_title,
                adventure.adventure_headline,
                narrator,
                adventure.adventure_tone,
                lore,
                adventure_created_at_utc,
                adventure_updated_at_utc,
            );

            let adventure_tags = parse_json_array(Some(adventure.adventure_tags.as_str()))?;

            new_adventure.add_character(character);
            new_adventure.add_tags(adventure_tags);
            new_adventures.insert(adventure_id_uuid, new_adventure);
        }

        Ok(new_adventures
            .values()
            .map(|adventure| adventure.clone())
            .collect())
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
    .bind(&title)
    .bind(&headline)
    .bind(&tone)
    .fetch_one(&mut *conn)
    .await?;

    for tag in &tags {
        ensure_tag(conn, tag).await?;
        attach_tag(conn, id, tag).await?;
    }

    Ok(AdventureSummary::try_from(adventure_row)?)
}

pub async fn get_adventures(
    conn: &mut SqliteConnection,
) -> Result<Vec<Adventure>, PersistenceError> {
    let rows: Vec<SqliteRow> = sqlx::query(
        r#"
SELECT a.id                      AS adventure_id,
       a.title                   AS adventure_title,
       a.headline             AS adventure_headline,
       a.tone                   AS adventure_tone,
       a.created_at             AS adventure_created_at,
       a.updated_at             AS adventure_updated_at,
       a.last_opened_at          AS adventure_last_opened_at,
       COALESCE(at.tags, '[]')   AS adventure_tags,
       COALESCE(l.lore, '[]')    AS adventure_lore,
       c.id                      AS character_id,
       c.name                   AS character_name,
       c.description                AS character_description,
       c.backstory                AS character_backstory,
       c.status                   AS character_status,
       c.portrait_path AS character_portrait_path,
       c.is_player AS character_is_player,
       c.created_at AS character_created_at,
       c.updated_at     AS character_updated_at,
       COALESCE(ctag.tags, '[]') AS character_tags,
       COALESCE(ct.trait, '[]')  AS character_traits,
       COALESCE(crl.rels, '[]')  AS character_relations,
       n.id AS narrator_id, n.name AS narrator_name, n.tone AS narrator_tone
FROM adventure AS a
         LEFT JOIN (SELECT adventure_id,
                           '[' || GROUP_CONCAT('"' || tag || '"', ',') || ']' AS tags
                    FROM adventure_tag GROUP BY adventure_id) AS at
                   ON at.adventure_id = a.id
         LEFT JOIN (SELECT adventure_id,
                           '[' || GROUP_CONCAT('"' || text || '"', ',') || ']' AS lore
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
         LEFT JOIN (SELECT cr.character_id,
                           '[' || GROUP_CONCAT('"' || cr.relation || ': ' || o.name || '"', ',') || ']' AS rels
                    FROM character_relation cr
                             JOIN character o ON o.id = cr.related_character_id   -- adjust to your column name
                    GROUP BY cr.character_id) AS crl
                   ON crl.character_id = c.id
         LEFT JOIN narrator AS n ON n.adventure_id = a.id
ORDER BY a.updated_at DESC, c.name DESC;
"#,
    )
    .fetch_all(&mut *conn)
    .await?;

    let adventure_rows: Vec<CompleteAdventureRow> =
        rows.iter().map(CompleteAdventureRow::decode).collect();
    CompleteAdventureRow::to_adventure(adventure_rows)
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
