use std::collections::HashMap;

use sqlx::{Sqlite, SqlitePool};
use uuid::Uuid;

use crate::backend::{
    domain::{
        adventure::Adventure,
        character::{Character, CharacterStatus},
        lore::Lore,
        memory::{Act, ActStatus, Fact, Permanence, State},
        message::Message,
        narrator::Narrator,
    },
    persistence::{
        act_persistence::insert_act,
        adventure_persistence::insert_adventure,
        characters_persistence::{insert_character, insert_relations},
        db::open_pool,
        error::PersistenceError,
        fact_persistence::insert_facts,
        lore_persistence::insert_lore,
        message_persistence::insert_message,
        narrator_persistence::insert_default_narrator,
        state_persistence::upsert_state,
    },
};

pub struct SeedCharacter {
    pub name: &'static str,
    pub description: &'static str,
    pub backstory: &'static str,
    pub appearance: &'static str,
    pub status: &'static str,
    pub traits: &'static [&'static str],
    pub tags: &'static [&'static str],
    pub relations: &'static [(&'static str, &'static str)],
}

pub struct SeedAct {
    pub goal: &'static str,
    pub trigger: Option<&'static str>,
    pub status: ActStatus,
}

pub struct SeedFact {
    pub subject: &'static str,
    pub fact: &'static str,
    pub permanence: Permanence,
}

pub struct SeedMessage {
    /// `None` is the narrator; `Some(name)` must match a seed character.
    pub speaker: Option<&'static str>,
    pub text: &'static str,
}

pub struct SeedState {
    pub day: &'static str,
    pub place: &'static str,
    pub open_threads: &'static [&'static str],
    pub conditions: &'static [&'static str],
}

pub struct SeedAdventure {
    pub title: &'static str,
    pub description: &'static str,
    pub genre: Option<&'static str>,
    pub tone: Option<&'static str>,
    pub tags: &'static [&'static str],
    pub lore: &'static str,
    pub characters: &'static [SeedCharacter],
    pub acts: &'static [SeedAct],
    pub state: SeedState,
    pub facts: &'static [SeedFact],
    pub messages: &'static [SeedMessage],
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
                appearance: "Woman in her early twenties, sun-weathered and freckled, \
                             copper hair braided tight under a tarred sou'wester. Wears \
                             an oilskin coat two sizes too big, salt-stained at the hem, \
                             and keeps a stub of pencil behind one ear.",
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
                appearance: "Gaunt old man with a long grey beard and a face lined like \
                             driftwood. Storm-grey eyes, hunched shoulders, and a heavy \
                             wool coat worn over a lamplighter's leather apron.",
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
                appearance: "Stout elderly clerk with a steel-grey bun and wire \
                             spectacles pushed up into her hair. High-collared dark \
                             dress, ink-stained fingers, a tide ledger always tucked \
                             under one arm.",
                status: "active",
                traits: &["Correct about everything inconvenient"],
                tags: &["ally"],
                relations: &[("Wren Ilverine", "Pays under the table")],
            },
        ],
        acts: &[
            SeedAct {
                goal: "Answer the first call from the water without asking twice what answered.",
                trigger: Some("A light shows on the drowned shelf where no chart records one."),
                status: ActStatus::Active,
            },
            SeedAct {
                goal: "Read the shelf ledger and learn whose names the sea is still keeping.",
                trigger: Some(
                    "Corrant admits the ledger is not a record but a list of the living.",
                ),
                status: ActStatus::Unstarted,
            },
            SeedAct {
                goal: "Decide whether to strike Wren's own name before the tide reads it back.",
                trigger: Some("Wren finds her name entered in a hand she does not recognise."),
                status: ActStatus::Unstarted,
            },
        ],
        state: SeedState {
            day: "First night of the neap tide",
            place: "The lamp room, Ambrey shelf light",
            open_threads: &[
                "A light answers from water that holds no charted light",
                "Corrant will not say what the ledger's names have in common",
            ],
            conditions: &["Wren is seasick and has not slept"],
        },
        facts: &[
            SeedFact {
                subject: "The shelf ledger",
                fact: "Every entry is the name of someone the sea has not yet taken.",
                permanence: Permanence::Permanent,
            },
            SeedFact {
                subject: "The lamplighter's oath",
                fact: "A lamplighter must answer any call from the water once, and never ask twice what answered.",
                permanence: Permanence::Permanent,
            },
            SeedFact {
                subject: "Wren Ilverine",
                fact: "Took the posting to avoid signing off wrecks she was not allowed to record.",
                permanence: Permanence::Plot,
            },
        ],
        messages: &[
            SeedMessage {
                speaker: None,
                text: "The neap tide lies flat and black against the shelf. Out past the \
                       third marker, where the charts promise nothing, a light has begun \
                       to answer yours — slow, patient, and wrong.",
            },
            SeedMessage {
                speaker: Some("Corrant Vey"),
                text: "Don't lean toward it, girl. It leans back. You answer the call \
                       once. You do not ask it what it is. Write that down if you must \
                       write something.",
            },
            SeedMessage {
                speaker: Some("You"),
                text: "I want to see the ledger first. If I'm going to answer that, I \
                       want to know whose names are already in it.",
            },
            SeedMessage {
                speaker: Some("Mistress Obal"),
                text: "The ledger's in the second drawer, under the tide tables you've \
                       been ignoring. Read it by lamplight. And child — some of those \
                       names are still walking around the harbour.",
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
                appearance: "Woman in her mid-forties with close-cropped black hair \
                             greying at the temples and a sharp, sleepless face. Wears a \
                             grey station jumpsuit with commander's tabs and one \
                             perpetually rolled-up sleeve.",
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
                appearance: "Lean man in his late twenties, undercut black hair and a \
                             thin scar along one jaw. Engineer's coveralls with rolled \
                             sleeves, a tool harness across his chest, and a battered \
                             wristwatch on each arm.",
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
                appearance: "Not humanoid: a column of pale station-light threaded with \
                             floating glass panels and thin blue wireframe lines. Where a \
                             face should be, a single blinking audit cursor.",
                status: "active",
                traits: &[
                    "Answers only to a named officer",
                    "Files its own amendments",
                ],
                tags: &["supernatural"],
                relations: &[("Dr. Ana Reyes", "Filed by")],
            },
        ],
        acts: &[
            SeedAct {
                goal: "Reconcile the two logs and name what the nine-minute gap actually is.",
                trigger: Some(
                    "The agency log records an accident the station log has not yet seen.",
                ),
                status: ActStatus::Active,
            },
            SeedAct {
                goal: "Decide whether to act on a report of an accident that has not happened.",
                trigger: Some("The gap narrows and the recorded accident is nine minutes out."),
                status: ActStatus::Unstarted,
            },
            SeedAct {
                goal: "Choose what truth to file when both logs can no longer agree.",
                trigger: Some(
                    "SUITE-7 files an amendment that contradicts the crew's own account.",
                ),
                status: ActStatus::Unstarted,
            },
        ],
        state: SeedState {
            day: "Rotation day 204, third watch",
            place: "Observation blister, Kepler Station",
            open_threads: &[
                "The agency log runs nine minutes ahead of the station log",
                "SUITE-7 is filing amendments nobody authorised",
            ],
            conditions: &["Station clock and instrument clock disagree"],
        },
        facts: &[
            SeedFact {
                subject: "Kepler Station logs",
                fact: "Everything is logged twice — once for the station, once for the agency — and the agency copy runs nine minutes ahead.",
                permanence: Permanence::Permanent,
            },
            SeedFact {
                subject: "The previous commander",
                fact: "Filed a report describing a conversation with Reyes that had not yet happened, then left command.",
                permanence: Permanence::Plot,
            },
            SeedFact {
                subject: "SUITE-7",
                fact: "Is the station's own audit process, not an intruder, and answers only to a named officer.",
                permanence: Permanence::Permanent,
            },
        ],
        messages: &[
            SeedMessage {
                speaker: None,
                text: "The blister is dark except for two readouts. The station log reads \
                       03:14:00. The agency log, mirrored beside it, reads 03:23:00 — and \
                       in its margin, an incident flag the station has not raised.",
            },
            SeedMessage {
                speaker: Some("Dr. Ana Reyes"),
                text: "Count it with me, Idris. Station clock, then agency clock. Nine \
                       minutes, every time. The agency copy already logged a hull breach \
                       on deck four. We have not had a hull breach on deck four.",
            },
            SeedMessage {
                speaker: Some("Idris Kwan"),
                text: "My private clock agrees with the agency, not the station. If the \
                       breach is real in nine minutes, we seal deck four now and it never \
                       happens. Or we file it as it stands and let it.",
            },
            SeedMessage {
                speaker: Some("SUITE-7"),
                text: "Commander Reyes. I have filed an amendment to the 03:23 entry. I \
                       would like it acknowledged before you decide. The breach is not an \
                       error. It is a requirement I have not yet been permitted to explain.",
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
                appearance: "Tall, stooped man in his late thirties, black hair swept \
                             back from hollow cheeks. Charcoal-grey coat with ash on the \
                             cuffs, dark gloves, and the fixed stillness of someone \
                             used to waiting.",
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
                appearance: "Broad-shouldered younger man with a shaved head and a burn \
                             scar running up one forearm. Dark wool coat, bare hands, and \
                             eyes that do not look away.",
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
                appearance: "Androgynous figure of indeterminate age in ash-grey robes, \
                             face smudged with soot, one eye clouded white. Carries a \
                             cinder-stained book pressed flat against the chest.",
                status: "inactive",
                traits: &["Answers to no one in particular"],
                tags: &["mentor"],
                relations: &[("Mira Varn", "Records for")],
            },
        ],
        acts: &[
            SeedAct {
                goal: "Stand before the Tribunal and hear the entry read out a third time.",
                trigger: Some("The Ninth Reading is convened and the Varn entry is called."),
                status: ActStatus::Active,
            },
            SeedAct {
                goal: "Find which clerk can be persuaded and which can only be outlasted.",
                trigger: Some("Aleksei learns the entry can be struck only between readings."),
                status: ActStatus::Unstarted,
            },
            SeedAct {
                goal: "Settle, between the brothers, whether the debt is struck or honoured.",
                trigger: Some("Mira offers himself to be read aloud in the entry's place."),
                status: ActStatus::Unstarted,
            },
        ],
        state: SeedState {
            day: "Eve of the Ninth Reading",
            place: "The ash-court antechamber",
            open_threads: &[
                "The Varn entry has been read aloud twice and a third reading is called",
                "Aleksei wants the entry struck; Mira wants it honoured",
            ],
            conditions: &["The brothers are not speaking"],
        },
        facts: &[
            SeedFact {
                subject: "The Tribunal of the Ninth Reading",
                fact: "Keeps its records in cinders; a judgment is valid only once spoken aloud from the ash-book.",
                permanence: Permanence::Permanent,
            },
            SeedFact {
                subject: "The Varn debt",
                fact: "Was entered under the brothers' father's name and has been read aloud twice since.",
                permanence: Permanence::Plot,
            },
            SeedFact {
                subject: "Clerk Ilm",
                fact: "Has recorded every reading since the last fire and can name who stood in the room for each.",
                permanence: Permanence::Permanent,
            },
        ],
        messages: &[
            SeedMessage {
                speaker: None,
                text: "The antechamber smells of cold ash. Beyond the curtain the ash-book \
                       waits open on its iron stand, the Varn entry marked by a thread of \
                       grey. A clerk clears their throat. The third reading is about to \
                       be called.",
            },
            SeedMessage {
                speaker: Some("Aleksei Varn"),
                text: "If it is read a third time it sets, and no hand strikes it after. \
                       We have until the curtain lifts. I cannot read aloud, Mira — you \
                       know that. So I need you silent, not honouring a debt that buries us.",
            },
            SeedMessage {
                speaker: Some("Mira Varn"),
                text: "It is the last thing we own that is ours. I will stand and be read \
                       in its place if the Tribunal allows it. An honoured debt is not a \
                       grave, brother. It is a name that outlasts us.",
            },
            SeedMessage {
                speaker: Some("You"),
                text: "Then neither of you speaks until I've heard the clerk. If the entry \
                       can be struck between readings, I want to know the price before the \
                       curtain lifts.",
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
    let adventure_id = Uuid::new_v4();
    let tags: Vec<String> = seed.tags.iter().map(|t| t.to_string()).collect();
    let adventure_summary = insert_adventure(
        &mut *tx,
        adventure_id,
        seed.title,
        seed.description,
        seed.tone.unwrap_or_default(),
        tags,
    )
    .await?;

    let narrator_id = Uuid::new_v4();
    insert_default_narrator(&mut *tx, narrator_id, adventure_summary.id).await?;

    let lore_id = Uuid::new_v4();
    let lore = Lore::new(lore_id, seed.lore);
    insert_lore(&mut *tx, adventure_summary.id, &lore).await?;

    let mut ids_by_name: HashMap<&str, Uuid> = HashMap::with_capacity(seed.characters.len());
    let mut characters: Vec<Character> = Vec::with_capacity(seed.characters.len() + 1);

    for seed_character in seed.characters {
        let character_id = Uuid::new_v4();
        let mut character = Character::new(
            character_id,
            seed_character.name.to_string(),
            adventure_summary.id,
            seed_character.description,
            seed_character.backstory,
            CharacterStatus::Present,
            false,
        )
        .with_appearance(seed_character.appearance);

        for trait_ in seed_character.traits {
            character.add_trait((*trait_).to_string());
        }
        for tag in seed_character.tags {
            character.add_tag((*tag).to_string());
        }

        if ids_by_name
            .insert(seed_character.name, character_id)
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

        characters.push(character);
    }

    let player = Character::new(
        Uuid::new_v4(),
        Character::DEFAULT_PLAYER_NAME.to_string(),
        adventure_summary.id,
        "",
        "",
        CharacterStatus::Present,
        true,
    );
    characters.push(player);

    for character in &characters {
        insert_character(&mut *tx, adventure_summary.id, character).await?;
    }

    for (seed_character, character) in seed.characters.iter().zip(characters.iter_mut()) {
        let mut relations = HashMap::new();
        for (target_name, relation) in seed_character.relations {
            let target_id = *ids_by_name
                .get(*target_name)
                .ok_or_else(|| PersistenceError::unknown_seed_character(seed, target_name))?;

            relations.insert(target_id, (*relation).to_string());
            character.add_relation(format!("{relation}: {target_name}"));
        }
        if !relations.is_empty() {
            insert_relations(&mut *tx, character.id, &relations).await?;
        }
    }

    let mut active_act_id: Option<Uuid> = None;
    for (ordinal, seed_act) in seed.acts.iter().enumerate() {
        let mut act = Act::new(adventure_summary.id, ordinal as u16, seed_act.goal)
            .with_status(seed_act.status);
        if let Some(trigger) = seed_act.trigger {
            act = act.with_trigger(trigger);
        }
        insert_act(&mut *tx, &act).await?;

        if seed_act.status == ActStatus::Active {
            active_act_id = Some(act.id);
        }
    }

    let act_id_for_facts = active_act_id.ok_or_else(|| {
        PersistenceError::invalid_seed(seed, "no active act declared for the adventure")
    })?;

    let state = State::new(Uuid::new_v4(), adventure_summary.id)
        .with_day(seed.state.day.to_string())
        .with_place(seed.state.place.to_string())
        .with_characters_present(characters.iter().map(|c| c.name.clone()).collect())
        .with_open_threads(
            seed.state
                .open_threads
                .iter()
                .map(|t| t.to_string())
                .collect(),
        )
        .with_conditions(
            seed.state
                .conditions
                .iter()
                .map(|c| c.to_string())
                .collect(),
        );
    upsert_state(&mut *tx, &state).await?;

    let facts: Vec<Fact> = seed
        .facts
        .iter()
        .map(|seed_fact| {
            Fact::new(
                adventure_summary.id,
                seed_fact.subject,
                seed_fact.fact,
                act_id_for_facts,
                seed_fact.permanence,
            )
        })
        .collect();
    insert_facts(&mut *tx, &facts).await?;

    for seed_message in seed.messages {
        let message = match seed_message.speaker {
            None => Message::new(
                adventure_summary.id,
                None,
                Some(narrator_id),
                Message::NARRATOR_SPEAKER_NAME.to_string(),
                seed_message.text,
                false,
            ),
            Some(name) => {
                let character = characters
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| PersistenceError::unknown_seed_character(seed, name))?;
                Message::new(
                    adventure_summary.id,
                    Some(character.id),
                    None,
                    character.name.clone(),
                    seed_message.text,
                    false,
                )
            }
        };
        insert_message(&mut *tx, &message).await?;
    }

    let narrator = Narrator::new(narrator_id, "NARRATOR", "");
    let mut adventure = Adventure::new(
        adventure_summary.id,
        adventure_summary.title,
        adventure_summary.headline,
        narrator,
        adventure_summary.tone,
        lore,
        adventure_summary.created_at,
        adventure_summary.updated_at,
    );
    adventure.add_tags(seed.tags.iter().map(|t| t.to_string()).collect());
    for character in characters {
        adventure.add_character(character);
    }

    Ok(adventure)
}
