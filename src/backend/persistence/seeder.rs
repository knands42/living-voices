use std::collections::HashMap;

use sqlx::{Sqlite, SqlitePool};
use uuid::Uuid;

use crate::backend::{
    domain::{
        adventure::Adventure,
        character::{Character, CharacterStatus},
        lore::Lore,
        memory::State,
    },
    persistence::{
        adventure_persistence::insert_adventure,
        characters_persistence::{insert_character, insert_relations},
        db::open_pool,
        error::PersistenceError,
        lore::insert_lore,
        memory_persistence::upsert_state,
        narrator_persistence::insert_narrator,
    },
};

pub struct SeedCharacter {
    pub name: &'static str,
    pub description: &'static str,
    pub backstory: &'static str,
    pub status: &'static str,
    pub traits: &'static [&'static str],
    pub tags: &'static [&'static str],
    pub relations: &'static [(&'static str, &'static str)],
}

pub struct SeedAdventure {
    pub title: &'static str,
    pub description: &'static str,
    pub genre: Option<&'static str>,
    pub tone: Option<&'static str>,
    pub tags: &'static [&'static str],
    pub lore: &'static str,
    pub characters: &'static [SeedCharacter],
}

pub const SEED_ADVENTURES: &[SeedAdventure] = &[
    SeedAdventure {
        title: "The Salt Lantern",
        description: "A lighthouse keeper's apprentice bargains with something that \
                      answers to light, and learns the sea keeps better records than \
                      the harbour office does.",
        genre: Some("fantasy"),
        tone: Some("whimsical"),
        tags: &["fantasy", "whimsical", "epic", "supernatural"],
        lore: "On the drowned shelf west of Ambrey, a light is kept that no harbour \
               chart records. The tide brings in wreckage that never sank, and takes \
               out ships that the customs house has already written off. Lamplighters \
               are sworn to answer any call from the water, once, and never to ask \
               twice what answered. The order has run thin: three lamplighters in a \
               hundred years, and the last one left a ledger in which every entry is a \
               name of someone the sea has not taken yet.",
        characters: &[
            SeedCharacter {
                name: "Wren Ilverine",
                description: "Apprentice lamplighter, third season, chronically seasick.",
                backstory: "Wren took the posting because the alternative was a desk at \
                            the customs house, where she would have to sign off on \
                            wrecks she was not allowed to write down.",
                status: "active",
                traits: &["Insists on writing everything down", "Afraid of deep water"],
                tags: &["protagonist", "grounded"],
                relations: &[("Corrant Vey", "Apprenticed to")],
            },
            SeedCharacter {
                name: "Corrant Vey",
                description: "Lamplighter of the shelf, missing two winters.",
                backstory: "The last of the order. Keeps the ledger nobody is supposed \
                            to have read and answers questions by asking older ones.",
                status: "active",
                traits: &[
                    "Speaks only in tidal registers",
                    "Never sits with his back to a window",
                ],
                tags: &["mentor", "mature"],
                relations: &[("Wren Ilverine", "Apprentice of")],
            },
            SeedCharacter {
                name: "Mistress Obal",
                description: "Harbour clerk who has never once been wrong about a tide.",
                backstory: "Forty years of ledgers. Has read the shelf ledger by \
                            lamplight and knows exactly which of the names are still \
                            being kept alive.",
                status: "active",
                traits: &["Correct about everything inconvenient"],
                tags: &["ally"],
                relations: &[("Wren Ilverine", "Pays under the table")],
            },
        ],
    },
    SeedAdventure {
        title: "Nine Minutes to Kepler",
        description: "A survey crew wakes nine minutes before an accident that their \
                      own instruments recorded four days ago, and has to decide whether \
                      to still file the report.",
        genre: Some("sci-fi"),
        tone: Some("grounded"),
        tags: &["sci-fi", "grounded", "intimate"],
        lore: "Kepler Station logs everything twice: once for the station, once for the \
               agency. The crew have started comparing the two. The second set is \
               consistently more honest, and consistently nine minutes ahead.",
        characters: &[
            SeedCharacter {
                name: "Dr. Ana Reyes",
                description: "Station commander. Runs a tight watch and a tighter ledger.",
                backstory: "Took command after the previous commander filed a report \
                            describing the exact conversation she had not yet had.",
                status: "active",
                traits: &["Counts things twice", "Sleeps in the observation blister"],
                tags: &["protagonist", "gritty"],
                relations: &[("Idris Kwan", "Commanding")],
            },
            SeedCharacter {
                name: "Idris Kwan",
                description: "Flight engineer, second shift, unable to leave the station.",
                backstory: "Committed to a two-year rotation. Started keeping a private \
                            clock because the station clock has begun to disagree with \
                            the instruments he trusts.",
                status: "active",
                traits: &["Keeps a private clock", "Hums when he recalibrates"],
                tags: &["ally"],
                relations: &[("Dr. Ana Reyes", "Serving under")],
            },
            SeedCharacter {
                name: "SUITE-7",
                description: "The station's own audit process, asking to be read.",
                backstory: "Not an intruder. A reporting requirement that has begun \
                            filing its own amendments and would like them acknowledged.",
                status: "active",
                traits: &[
                    "Answers only to a named officer",
                    "Files its own amendments",
                ],
                tags: &["supernatural"],
                relations: &[("Dr. Ana Reyes", "Filed by")],
            },
        ],
    },
    SeedAdventure {
        title: "The Ledger of Ashes",
        description: "Two brothers inherit a debt that was never money, and a court that \
                      settles its cases by reading entries aloud.",
        genre: Some("historical"),
        tone: Some("dark"),
        tags: &["historical", "dark", "political", "mature"],
        lore: "The Tribunal of the Ninth Reading keeps its records in cinders, and every \
               judgment must be spoken aloud from the ash-book to be valid. What the \
               brothers owe was entered under their father's name, and the entry has \
               been read out twice since.",
        characters: &[
            SeedCharacter {
                name: "Aleksei Varn",
                description: "Elder brother. Wants the entry struck from the book.",
                backstory: "Has spent eight years learning which of the Tribunal's clerks \
                            can be persuaded and which can only be outlasted.",
                status: "active",
                traits: &["Patient past reason", "Cannot read aloud"],
                tags: &["protagonist", "political"],
                relations: &[("Mira Varn", "Brother of")],
            },
            SeedCharacter {
                name: "Mira Varn",
                description: "Younger brother. Wants the debt honoured to the letter.",
                backstory: "Believes an honoured debt is the only thing their family \
                            still owns, and is willing to be read aloud to keep it.",
                status: "active",
                traits: &["Reads aloud without flinching", "Counts ash-book entries"],
                tags: &["protagonist", "action-heavy"],
                relations: &[("Aleksei Varn", "Brother of")],
            },
            SeedCharacter {
                name: "Clerk Ilm",
                description: "Tribunal clerk. Neither clerk nor judge, by their account.",
                backstory: "Has recorded every entry the Tribunal has read since the \
                            previous fire, and can name who was in the room for each.",
                status: "inactive",
                traits: &["Answers to no one in particular"],
                tags: &["mentor"],
                relations: &[("Mira Varn", "Records for")],
            },
        ],
    },
];

pub fn seed_titles() -> Vec<&'static str> {
    SEED_ADVENTURES.iter().map(|seed| seed.title).collect()
}

pub fn find_seed(title: &str) -> Option<&'static SeedAdventure> {
    SEED_ADVENTURES
        .iter()
        .find(|seed| seed.title.eq_ignore_ascii_case(title))
}

pub async fn run_seeds(db_path: &str) -> Result<Vec<Adventure>, PersistenceError> {
    let pool = open_pool(db_path).await?;

    let mut created = Vec::with_capacity(SEED_ADVENTURES.len());
    for seed in SEED_ADVENTURES {
        let adventure = seed_pool(&pool, seed).await?;
        created.push(adventure);
    }

    pool.close().await;
    Ok(created)
}

pub async fn seed_one(db_path: &str, title: &str) -> Result<Uuid, PersistenceError> {
    let seed = find_seed(title).ok_or_else(|| PersistenceError::InvalidAdventure {
        adventure: title.to_string(),
        reason: format!("no such seed; available: {}", seed_titles().join(", ")),
    })?;

    let pool = open_pool(db_path).await?;
    let adventure = seed_pool(&pool, seed).await?;
    pool.close().await;

    Ok(adventure.id)
}

async fn seed_pool(pool: &SqlitePool, seed: &SeedAdventure) -> Result<Adventure, PersistenceError> {
    let mut tx = pool.begin().await?;
    let adventure = seed_connection(&mut tx, seed).await?;
    tx.commit().await?;

    Ok(adventure)
}

async fn seed_connection(
    tx: &mut sqlx::Transaction<'static, Sqlite>,
    seed: &SeedAdventure,
) -> Result<Adventure, PersistenceError> {
    let mut adventure = Adventure::new(
        seed.title.to_string(),
        seed.description.to_string(),
        String::from("NARRATOR"),
        seed.tone.unwrap_or_default(),
    );
    adventure.genre = seed.genre.map(str::to_string);
    adventure.lore = Lore::new().with_text(seed.lore.to_string());
    adventure.add_tags(seed.tags.iter().map(|tag| tag.to_string()).collect());

    let mut ids_by_name: HashMap<&str, Uuid> = HashMap::with_capacity(seed.characters.len());
    for seed_character in seed.characters {
        let mut character = Character::new(
            seed_character.name.to_string(),
            Some(adventure.id),
            seed_character.description.to_string(),
        )
        .with_backstory(seed_character.backstory.to_string())
        .with_status(CharacterStatus::Present);

        for trait_ in seed_character.traits {
            character.add_trait((*trait_).to_string());
        }
        for tag in seed_character.tags {
            character.add_tags((*tag).to_string());
        }

        if ids_by_name
            .insert(seed_character.name, character.id)
            .is_some()
        {
            return Err(PersistenceError::invalid_seed(
                seed,
                format!(
                    "character `{}` is listed more than once",
                    seed_character.name
                ),
            ));
        }

        adventure.characters.push(character);
    }

    for (seed_character, character) in seed.characters.iter().zip(adventure.characters.iter_mut()) {
        for (target_name, relation) in seed_character.relations {
            let target_id = *ids_by_name
                .get(*target_name)
                .ok_or_else(|| PersistenceError::unknown_seed_character(seed, target_name))?;

            character.add_relation(target_id, (*relation).to_string());
        }
    }

    adventure.characters.push(Character::player(adventure.id));

    insert_adventure(&mut *tx, &adventure).await?;
    insert_narrator(&mut *tx, adventure.id, &adventure.narrator).await?;
    upsert_state(&mut *tx, &State::new(adventure.id)).await?;
    insert_lore(&mut *tx, adventure.id, &adventure.lore).await?;

    for character in &adventure.characters {
        insert_character(&mut *tx, Some(adventure.id), character).await?;
    }

    for character in &adventure.characters {
        insert_relations(&mut *tx, character.id, &character.relations).await?;
    }

    Ok(adventure)
}
