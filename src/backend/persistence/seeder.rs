use std::collections::HashMap;

use sqlx::{Sqlite, SqlitePool};
use uuid::Uuid;

use crate::backend::domain::adventure::AdventureStatus;
use crate::backend::persistence::adventure_persistence::update_adventure;
use crate::backend::{
    domain::{
        adventure::Adventure,
        character::{Character, CharacterStatus},
        lore::Lore,
        memory::{Act, ActStatus, CharacterCondition, Fact, State},
        message::Message,
        narrator::Narrator,
    },
    persistence::{
        act_persistence::upsert_act, adventure_persistence::insert_adventure,
        characters_persistence::insert_character, db::open_pool, error::PersistenceError,
        fact_persistence::upsert_facts, lore_persistence::insert_lore,
        message_persistence::insert_messages, narrator_persistence::insert_default_narrator,
        state_persistence::upsert_state,
    },
};

pub struct SeedCharacter {
    pub name: &'static str,
    pub genre: &'static str,
    pub description: &'static str,
    pub backstory: &'static str,
    pub identity_prompt: &'static str,
    pub current_outfit: &'static str,
    pub status: &'static str,
    pub traits: &'static [&'static str],
    pub tags: &'static [&'static str],
    pub relations: &'static [(&'static str, &'static str)],
    /// A two-exchange transcript in the character's voice. Empty means the seed
    /// ships without one and the prompt falls back to traits alone.
    pub message_example: &'static str,
}

pub struct SeedAct {
    pub goal: &'static str,
    pub trigger: Option<&'static str>,
    pub status: ActStatus,
}

pub struct SeedFact {
    pub subject: &'static str,
    pub fact: &'static str,
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
    pub character_conditions: &'static [(&'static str, &'static str)],
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
        description: "A lighthouse keeper's apprentice works a night shift with a broken \
                      radio, an overdue ship and a logbook that has to be correct.",
        genre: Some("drama"),
        tone: Some("calm"),
        tags: &["drama", "calm", "slice-of-life"],
        lore: "Ambrey Point lighthouse runs three shifts a day. The logbook records every \
               ship sighted, every lamp failure and every radio call. Keepers write each \
               entry when it happens. A late entry is treated the same as a missing one.",
        characters: &[
            SeedCharacter {
                name: "Wren Ilverine",
                genre: "female",
                description: "Apprentice at the lighthouse, third month on the job.",
                backstory: "Wren took the post because it came with housing. She is still \
                            learning the logbook rules and does not like the night shift \
                            on her own.",
                identity_prompt: "young woman in her early twenties, sun-weathered \
                                  freckled skin, copper hair braided tight, sharp grey \
                                  eyes, wiry build, painterly illustration, warm lamplight",
                current_outfit: "tarred sou'wester hat over the braid, oversized oilskin \
                                 coat two sizes too big, hem frayed and salt-stained, \
                                 stub of pencil tucked behind one ear",
                status: "active",
                traits: &[
                    "Writes everything down",
                    "Does not like being alone at night",
                ],
                tags: &["protagonist", "grounded"],
                relations: &[("Corrant Vey", "Apprenticed to")],
                message_example: r#"You: "The radio is dead."

Wren: *turns the dial twice, then opens the logbook* "Dead since 22:10. I turned it off and on again. It stayed dead."

You: "Did you check the battery?"

Wren: "There is no battery. The holder is empty." *writes the time in the margin* "That is two problems, and I only have one pencil.""#,
            },
            SeedCharacter {
                name: "Corrant Vey",
                genre: "male",
                description: "Senior keeper, on the phone from the harbour office.",
                backstory: "Corrant has run the lighthouse for eleven years. He is at the \
                            harbour office tonight and cannot leave until the weather clears.",
                identity_prompt: "gaunt old man, long grey beard, face lined like \
                                  driftwood, storm-grey eyes, weathered skin, stooped \
                                  build, painterly illustration, dim lamplight",
                current_outfit: "heavy wool coat worn over a lamplighter's leather apron, \
                                 worn leather gloves",
                status: "active",
                traits: &["Checks the logbook first", "Speaks in short sentences"],
                tags: &["mentor", "mature"],
                relations: &[("Wren Ilverine", "Apprentice of")],
                message_example: r#"You: "The radio is dead."

Corrant: "Did you write down the time?"

You: "22:10."

Corrant: "Good. Now check the battery holder. If it is empty, we have a problem, not an excuse." *the phone clicks as he moves it to the other ear* "Read the entry back to me exactly as you wrote it.""#,
            },
            SeedCharacter {
                name: "Mistress Obal",
                genre: "female",
                description: "Harbour clerk. Keeps the keys to the supply cabinet.",
                backstory: "Obal has worked at the harbour office for forty years. She signs \
                            out the lighthouse supplies and knows where everything is kept.",
                identity_prompt: "stout elderly clerk, steel-grey bun, round lined face, \
                                  wire spectacles pushed up into her hair, ink-stained \
                                  fingers, painterly illustration",
                current_outfit: "high-collared dark dress, a ledger tucked under one arm",
                status: "active",
                traits: &[
                    "Keeps the keys",
                    "Does not lend supplies without a signature",
                ],
                tags: &["ally"],
                relations: &[("Wren Ilverine", "Signs out her supplies")],
                message_example: r#"You: "We need a new radio battery."

Obal: "Sign for it. Date, time and initials."

You: "Can I sign without the form?"

Obal: *slides the form across the desk and taps the first line* "No. Fill it in and I can sign out the battery." ((the form is one line long))"#,
            },
        ],
        acts: &[
            SeedAct {
                goal: "Get the radio working before the next ship calls the lighthouse.",
                trigger: Some("The radio dies at the start of the night shift."),
                status: ActStatus::Active,
            },
            SeedAct {
                goal: "Record the ship Halvard as overdue once the radio works again.",
                trigger: Some("The radio comes back and the Halvard has not called."),
                status: ActStatus::Unstarted,
            },
            SeedAct {
                goal: "Decide who signs the logbook for the night shift.",
                trigger: Some("Corrant cannot get back before morning."),
                status: ActStatus::Unstarted,
            },
        ],
        state: SeedState {
            day: "Night shift, 22:10",
            place: "Lamp room, Ambrey Point lighthouse",
            open_threads: &[
                "The radio died at 22:10 and the battery holder is empty",
                "The ship Halvard is due at 22:30 and has not called",
            ],
            character_conditions: &[("Wren Ilverine", "cold hands, third hour without a break")],
        },
        facts: &[
            SeedFact {
                subject: "The radio",
                fact: "Is dead because the battery holder is empty.",
            },
            SeedFact {
                subject: "The ship Halvard",
                fact: "Is due at 22:30 and has not made its radio call.",
            },
            SeedFact {
                subject: "The lighthouse logbook",
                fact: "Records every ship, lamp failure and radio call with the exact time.",
            },
        ],
        messages: &[
            SeedMessage {
                speaker: None,
                text: "The lamp room is warm and the lamp is lit. The clock on the wall \
                       reads 22:10. The radio makes one short burst of static and then \
                       goes quiet.",
            },
            SeedMessage {
                speaker: Some("Wren Ilverine"),
                text: "*turns the volume dial twice* The radio is dead. Not quiet, dead.",
            },
            SeedMessage {
                speaker: None,
                text: "Wren writes the time in the logbook. She holds the pencil over the \
                       page and waits for the next thing to happen.",
            },
            SeedMessage {
                speaker: Some("Corrant Vey"),
                text: "*on the phone from the harbour office* Read me the logbook entry \
                       first. Then tell me what the radio is doing, in that order.",
            },
            SeedMessage {
                speaker: Some("Wren Ilverine"),
                text: "22:10, radio dead. 22:12, turned the dial, no sound. 22:14, checked \
                       the battery holder, it is empty.",
            },
            SeedMessage {
                speaker: Some("Corrant Vey"),
                text: "Call Obal at the harbour office. The spare battery is in the supply \
                       cabinet. Sign for it and bring it back.",
            },
            SeedMessage {
                speaker: Some("Mistress Obal"),
                text: "*on the phone* I can sign out one battery. I need the time, your \
                       initials and the word urgent on the line.",
            },
            SeedMessage {
                speaker: Some("Wren Ilverine"),
                text: "The clock says 22:40. The Halvard should have called at 22:30. It \
                       has not called.",
            },
            SeedMessage {
                speaker: None,
                text: "Obal comes up to the lamp room with the battery. The radio comes \
                       back on at 22:55. The first call it picks up is the Halvard, \
                       twenty-five minutes late.",
            },
            SeedMessage {
                speaker: Some("Corrant Vey"),
                text: "Write the call down with the real time and the real reason. Late \
                       call, late entry, both in the book.",
            },
        ],
    },
    SeedAdventure {
        title: "The Ghost Light",
        description: "A harbour keeper and a salvage captain row out to a ship that sank \
                      ten years ago. They carry one rusted candle that has to be lit on \
                      its deck.",
        genre: Some("supernatural"),
        tone: Some("calm"),
        tags: &["supernatural", "calm", "short"],
        lore: "The Mercy sank ten years ago with all hands and no lantern. It returns at \
               night, dark and silent. The harbour office keeps one rusted candle from the \
               ship. The rule is to light it on the Mercy's deck. Then the ship goes back \
               under.",
        characters: &[
            SeedCharacter {
                name: "Nell Rourke",
                genre: "female",
                description: "Harbour keeper. Keeps the rusted candle in the office.",
                backstory: "Nell has kept the harbour records for ten years. She has the \
                            only candle from the Mercy and knows the rule for lighting it.",
                identity_prompt: "woman in her forties, dark hair tied back, weathered \
                                  face, wool coat, painterly illustration, night harbour \
                                  light",
                current_outfit: "heavy wool coat over a harbour uniform, lantern in one \
                                 hand",
                status: "active",
                traits: &["Follows the harbour rules", "Keeps the candle box locked"],
                tags: &["protagonist", "grounded"],
                relations: &[("Dell Marrow", "Hired him for the crossing")],
                message_example: r#"You: "Do we light it on the dock?"

Nell: "No. It has to be lit on the ship's deck. That is the rule."

You: "And if we light it here?"

Nell: *closes the candle box* "Then we waste the only candle and the Mercy comes back tomorrow." ((one candle, one trip))"#,
            },
            SeedCharacter {
                name: "Dell Marrow",
                genre: "male",
                description: "Salvage captain. Rows the boat and asks questions.",
                backstory: "Dell salvages wrecks along the coast. He took the crossing for \
                            the fee and does not believe in ghosts.",
                identity_prompt: "broad man in his fifties, grey beard, oilskin coat, rope \
                                  burns on his hands, painterly illustration, night harbour \
                                  light",
                current_outfit: "oilskin coat, wool cap, oars over one shoulder",
                status: "active",
                traits: &["Does not believe in ghosts", "Rows without complaining"],
                tags: &["ally"],
                relations: &[("Nell Rourke", "Works for her")],
                message_example: r#"You: "Are you coming aboard?"

Dell: *takes the oars* "Somebody has to row. You take the candle and do not drop it."

You: "What about the ghost?"

Dell: "The ghost wants the candle lit. That makes him the only one here on our side." ((as far as I know))"#,
            },
            SeedCharacter {
                name: "Tam Voss",
                genre: "male",
                description: "Ghost of the Mercy's keeper.",
                backstory: "Tam went down with the Mercy ten years ago. He stays on the deck \
                            until the rusted candle is lit.",
                identity_prompt: "pale gaunt man, wet grey coat, dark hair plastered to his \
                                  face, hollow eyes, faintly translucent, painterly \
                                  illustration, moonlight",
                current_outfit: "rusted keeper's coat, soaked through, no hat",
                status: "active",
                traits: &["Speaks plainly", "Cannot leave the ship"],
                tags: &["ally"],
                relations: &[("Nell Rourke", "Knows her by name")],
                message_example: r#"You: "Are you the one who keeps the light?"

Tam: *stands in the bow, grey and wet* "I was the keeper. Ten years and the candle never came back."

You: "The holder is rusted."

Tam: "Turn the candle a quarter turn. Then light it and I can go." ((please))"#,
            },
        ],
        acts: &[
            SeedAct {
                goal: "Light the rusted candle on the deck of the ghost ship.",
                trigger: Some("The rusted candle is seated in the holder and the ship waits."),
                status: ActStatus::Active,
            },
            SeedAct {
                goal: "Row back to the harbour before the Mercy goes under.",
                trigger: Some("The candle is lit and the Mercy starts to sink."),
                status: ActStatus::Unstarted,
            },
            SeedAct {
                goal: "Record the Mercy's last voyage in the harbour log.",
                trigger: Some("Nell asks what to write in the log."),
                status: ActStatus::Unstarted,
            },
        ],
        state: SeedState {
            day: "Night, low tide",
            place: "Foredeck of the Mercy",
            open_threads: &[
                "The rusted candle is in the holder but not lit",
                "The Mercy returns every night until the candle burns",
            ],
            character_conditions: &[("Nell Rourke", "cold hands from the rowing")],
        },
        facts: &[
            SeedFact {
                subject: "The rusted candle",
                fact: "Is the only candle from the Mercy. It must be lit on the ship's deck \
                       to send the ship back under.",
            },
            SeedFact {
                subject: "The ghost ship Mercy",
                fact: "Sank ten years ago with all hands and no lantern. It returns at night.",
            },
            SeedFact {
                subject: "Tam Voss",
                fact: "Is the ghost of the Mercy's keeper. He stays on the deck until the \
                       candle is lit.",
            },
        ],
        messages: &[
            SeedMessage {
                speaker: None,
                text: "The harbour is dark. The Mercy sits forty metres out with no sails \
                       and no lantern. Nell sets the rusted candle box on the dock.",
            },
            SeedMessage {
                speaker: Some("Nell Rourke"),
                text: "The candle is from the Mercy. It has been in the harbour office for \
                       ten years. It goes in the holder on the foredeck.",
            },
            SeedMessage {
                speaker: Some("Dell Marrow"),
                text: "Ten years and no one has lit it?",
            },
            SeedMessage {
                speaker: Some("Nell Rourke"),
                text: "Ten years and no one would row out. One candle, one trip.",
            },
            SeedMessage {
                speaker: None,
                text: "Dell takes the oars. A man appears in the bow of the boat, grey and \
                       wet, sitting where no one sat a moment ago.",
            },
            SeedMessage {
                speaker: Some("Tam Voss"),
                text: "The lantern stand is on the foredeck. The candle goes in the rusted \
                       holder. Do not light it before it is seated.",
            },
            SeedMessage {
                speaker: Some("Dell Marrow"),
                text: "You are the ship's keeper.",
            },
            SeedMessage {
                speaker: Some("Tam Voss"),
                text: "I was. I am still on the ship. Light the candle and we are both done.",
            },
            SeedMessage {
                speaker: None,
                text: "The boat reaches the Mercy. The rope is tied. The deck is wet but \
                       solid. The rusted holder is at the bow, empty.",
            },
            SeedMessage {
                speaker: Some("Dell Marrow"),
                text: "The candle does not fit. It is too wide.",
            },
            SeedMessage {
                speaker: Some("Tam Voss"),
                text: "Turn it a quarter turn clockwise. The holder is warped.",
            },
            SeedMessage {
                speaker: None,
                text: "The candle seats with a click. Nell passes you the lighter. The wind \
                       drops and the ship is quiet.",
            },
            SeedMessage {
                speaker: Some("Nell Rourke"),
                text: "The candle is in the holder. Light it and the Mercy goes back under.",
            },
            SeedMessage {
                speaker: Some("Tam Voss"),
                text: "Light it. I will wait.",
            },
        ],
    },
    SeedAdventure {
        title: "The Ledger of Ashes",
        description: "Two brothers go to court over a family debt. The court only accepts \
                      a debt that has been read aloud.",
        genre: Some("historical"),
        tone: Some("dark"),
        tags: &["historical", "dark", "political", "mature"],
        lore: "The Tribunal of the Ninth Reading keeps one ledger of debts. A debt is valid \
               only after it has been read aloud in court. A debt read three times cannot \
               be struck from the ledger.",
        characters: &[
            SeedCharacter {
                name: "Aleksei Varn",
                genre: "male",
                description: "Elder brother. Wants the court to strike the debt.",
                backstory: "Aleksei has spent eight years trying to have the debt struck. \
                            He cannot read aloud in court, so Mira has to read for him.",
                identity_prompt: "tall stooped man in his late thirties, black hair swept \
                                  back from hollow cheeks, pale watchful eyes, long drawn \
                                  face, dark historical painterly illustration",
                current_outfit: "charcoal-grey coat with ash on the cuffs, dark gloves",
                status: "active",
                traits: &["Will not read aloud", "Keeps a copy of every court entry"],
                tags: &["protagonist", "political"],
                relations: &[("Mira Varn", "Brother of")],
                message_example: r#"You: "They will read it out again."

Aleksei: "They will. And I will sit through the third reading, and the fourth, and not one of them will have said it right."

You: "Then read it yourself and be done with it."

Aleksei: *a long pause, then quieter than before* "I cannot. Eight years, and I still cannot put the words in the order they are written.""#,
            },
            SeedCharacter {
                name: "Mira Varn",
                genre: "male",
                description: "Younger brother. Wants the debt paid, not struck.",
                backstory: "Mira believes the family should pay what their father owed. He \
                            has read the entry aloud twice already.",
                identity_prompt: "broad-shouldered young man, shaved head, burn scar \
                                  running up one forearm, unwavering pale eyes, square jaw, \
                                  dark historical painterly illustration",
                current_outfit: "dark wool coat, bare hands",
                status: "active",
                traits: &["Reads aloud in court", "Keeps the debt record"],
                tags: &["protagonist", "action-heavy"],
                relations: &[("Aleksei Varn", "Brother of")],
                message_example: r#"You: "Aleksei cannot say it. You can."

Mira: *opens the entry and reads without stopping* "And the house of Varn was found owing." *looks up* "There. It has been said in the room and they cannot pretend otherwise."

You: "That is not honouring it."

Mira: "Four hundred and six entries. I have counted them." *closes the book* "I will count them again tomorrow, and the debt will still be the only thing we own.""#,
            },
            SeedCharacter {
                name: "Clerk Ilm",
                genre: "androgynous",
                description: "Court clerk. Records every reading.",
                backstory: "Ilm has recorded every court reading for eleven years. They do \
                            not speak for the Tribunal.",
                identity_prompt: "androgynous figure of indeterminate age, face smudged \
                                  with soot, one eye clouded white, gaunt build, dark \
                                  historical painterly illustration",
                current_outfit: "ash-grey robes, ledger pressed flat against the chest",
                status: "inactive",
                traits: &["Records the readings", "Answers in short sentences"],
                tags: &["mentor"],
                relations: &[("Mira Varn", "Records for")],
                message_example: r#"You: "Do you speak for the Tribunal?"

Clerk Ilm: *keeps writing, one clouded eye on the page and the other somewhere else* "I record for it. Speaking is a different office."

You: "Then who do I ask about the Varn entry?"

Clerk Ilm: "You ask the room, and the room gets read to you." *presses the ledger flat against the chest* "I was present for nine readings. I can name everyone else who was. I cannot tell you what it means.""#,
            },
        ],
        acts: &[
            SeedAct {
                goal: "Hear the third reading and decide whether the debt is struck or paid.",
                trigger: Some("The court calls the third reading of the Varn debt."),
                status: ActStatus::Active,
            },
            SeedAct {
                goal: "Find out which clerk can change the ledger before the third reading ends.",
                trigger: Some("Aleksei learns the debt can only be struck between readings."),
                status: ActStatus::Unstarted,
            },
            SeedAct {
                goal: "Choose between the brothers how the debt ends.",
                trigger: Some("Mira offers to be read aloud in place of the debt."),
                status: ActStatus::Unstarted,
            },
        ],
        state: SeedState {
            day: "Morning of the third reading",
            place: "Antechamber of the Tribunal",
            open_threads: &[
                "The third reading of the Varn debt is about to be called",
                "Aleksei wants the debt struck and Mira wants it paid",
            ],
            character_conditions: &[("Aleksei Varn", "will not look at his brother")],
        },
        facts: &[
            SeedFact {
                subject: "The Tribunal ledger",
                fact: "Records every debt in the city. A debt is valid once read aloud and \
                       permanent after three readings.",
            },
            SeedFact {
                subject: "The Varn debt",
                fact: "Was entered under the brothers' father and has been read aloud twice.",
            },
            SeedFact {
                subject: "Clerk Ilm",
                fact: "Has recorded every reading for eleven years and can name who was in \
                       court for each.",
            },
        ],
        messages: &[
            SeedMessage {
                speaker: None,
                text: "The antechamber is cold and quiet. Through the curtain the court \
                       ledger is open on its stand. The Varn entry has been read aloud \
                       twice. A clerk clears their throat and the room goes quiet.",
            },
            SeedMessage {
                speaker: Some("Aleksei Varn"),
                text: "If the entry is read a third time, the debt becomes permanent and \
                       no one can strike it. We have until the curtain opens. I cannot \
                       read it aloud, Mira. Stay silent.",
            },
            SeedMessage {
                speaker: Some("Mira Varn"),
                text: "The debt is the last thing our family owns. I will read it a third \
                       time and pay it. If the court lets me read it in father's place, \
                       the debt is paid and the name stays clean.",
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
            seed_character.genre.to_string(),
            adventure_summary.id,
            seed_character.description,
            seed_character.backstory,
            CharacterStatus::Present,
            false,
        )
        .with_appearance(seed_character.identity_prompt)
        .with_appearance_current_outfit(seed_character.current_outfit);

        for trait_ in seed_character.traits {
            character.add_trait((*trait_).to_string());
        }
        for tag in seed_character.tags {
            character.add_tag((*tag).to_string());
        }

        if !seed_character.message_example.is_empty() {
            character.message_example = Some(seed_character.message_example.to_string());
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
        Character::PLACEHOLDER_GENRE.to_string(),
        adventure_summary.id,
        "",
        "",
        CharacterStatus::Present,
        true,
    );
    characters.push(player);

    // Relations are free text on the character row, so they must be set before the
    // insert; the cast is only used to catch a seed that points at an unknown name.
    for (seed_character, character) in seed.characters.iter().zip(characters.iter_mut()) {
        for (target_name, relation) in seed_character.relations {
            if !ids_by_name.contains_key(*target_name) {
                return Err(PersistenceError::unknown_seed_character(seed, target_name));
            }

            character.add_relation(format!("{relation}: {target_name}"));
        }
    }

    for character in &characters {
        insert_character(&mut *tx, adventure_summary.id, character).await?;
    }

    let mut active_act_id: Option<Uuid> = None;
    for (ordinal, seed_act) in seed.acts.iter().enumerate() {
        let mut act = Act::new(adventure_summary.id, ordinal as u16, seed_act.goal)
            .with_status(seed_act.status);
        if let Some(trigger) = seed_act.trigger {
            act = act.with_trigger(trigger);
        }
        upsert_act(&mut *tx, &act).await?;

        if seed_act.status == ActStatus::Active {
            active_act_id = Some(act.id);
        }
    }

    active_act_id.ok_or_else(|| {
        PersistenceError::invalid_seed(seed, "no active act declared for the adventure")
    })?;

    let state = State::new(Uuid::new_v4(), adventure_summary.id)
        .with_day(seed.state.day.to_string())
        .with_place(seed.state.place.to_string())
        .with_characters_present(
            characters
                .iter()
                .filter_map(|c| (!c.is_player).then_some(c.name.clone()))
                .collect(),
        )
        .with_open_threads(
            seed.state
                .open_threads
                .iter()
                .map(|t| t.to_string())
                .collect(),
        )
        .with_character_conditions(
            seed.state
                .character_conditions
                .iter()
                .map(|(character, note)| CharacterCondition::new(*character, *note))
                .collect(),
        );
    upsert_state(&mut *tx, &state).await?;

    let facts: Vec<Fact> = seed
        .facts
        .iter()
        .map(|seed_fact| Fact::new(seed_fact.subject, seed_fact.fact))
        .collect();
    upsert_facts(&mut *tx, adventure_summary.id, &facts).await?;

    let mut seeded_messages = Vec::with_capacity(seed.messages.len());
    for seed_message in seed.messages {
        let message = match seed_message.speaker {
            None => Message::new(
                adventure_summary.id,
                None,
                Some(narrator_id),
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
                    seed_message.text,
                    false,
                )
            }
        };

        seeded_messages.push(message);
    }
    insert_messages(&mut *tx, &seeded_messages).await?;

    let narrator = Narrator::new(narrator_id, adventure_id, "NARRATOR", "");
    let mut adventure = Adventure::new(
        adventure_summary.id,
        adventure_summary.title,
        adventure_summary.headline,
        narrator,
        adventure_summary.tone,
        lore,
        AdventureStatus::Private,
        adventure_summary.created_at,
        adventure_summary.updated_at,
    );
    adventure.add_tags(seed.tags.iter().map(|t| t.to_string()).collect());
    for character in characters {
        adventure.add_character(character);
    }

    update_adventure(&mut *tx, &adventure).await?;
    Ok(adventure)
}
