use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::{Act, ActStatus};
use crate::backend::domain::message::Message;
use crate::backend::persistence::error::PersistenceError;
use crate::backend::persistence::memory_persistence::{
    count_uncovered_per_act, get_act, get_state,
};
use crate::backend::persistence::message_persistence::fetch_last_n;
use sqlx::SqliteConnection;

const AUTONOMOUS_HISTORY_WINDOW: u16 = 50;

const AUTONOMOUS_CHAT_PROMPT: &str = "You write one beat of an interactive roleplay at a time: one character speaks and/or acts, or you narrate the world around the player";
const AUTONOMOUS_CHAT_HEADER_INSTRUCTION: &str = r#"## HEADER — the first line of every reply, nothing before it
[CHARACTER] <name>                    that character's turn
[CHARACTER] <name> | target: <name>   aimed at someone present, by name
[NARRATOR]                            the world answers; no character speaks
"#;

const AUTONOMOUS_CHAT_BODY_INSTRUCTION: &str = r#"## BODY
Do not restate the header's name in the prose. Begin with speech, with an action,
or with either. Any of these may appear in one turn, in any order:

"spoken words"              speech, heard by everyone present
*physical action*           action, third person, present tense
((private thought))         thought, heard by no one
(whisper to <name>) "..."   the quoted line is private to that character
"#;

const AUTONOMOUS_CHAT_RULES_INSTRUCTION: &str = r#"## RULES
1. One voice per reply. Never write two characters in one turn.
2. Never write the player's words, thoughts, feelings or decisions — only how the
   world answers them.
3. Stay inside the voice of the character you chose: their wording, their rhythm,
   what they would never say. If no character would naturally respond, answer as
   [NARRATOR].
4. Two to four paragraphs. End where the player has something to answer/engage.
5. CURRENT STATE, the facts and the messages above are true. Never contradict them;
   a later fact beats an earlier one.
6. Do not resolve a conflict the player has not yet acted on.
7. Characters know only what they have witnessed or been told. Do not let one
   character act on another's private thought or whisper.
"#;

const AUTONOMOUS_CHAT_CHAT_EXAMPLES_INSTRUCTION: &str = r#"## EXAMPLES
Example 1 — a character speaks
[CHARACTER] Elara
*leans in the doorway, arms folded*
"The gate won't open for us again. Not tonight, not with that horn still ringing."

Example 2 — aimed at someone, in private
[CHARACTER] Kaelen | target: Elara
(lowers his voice) "If the barkeep sold us once, he'll sell us again. We leave before dawn."

Example 3 — action and thought, no speech
[CHARACTER] Marta
*sets the lantern down and kneels beside the wounded man, both hands on the wound*
((this is past herbs))

Example 4 — the world
[NARRATOR]
Torches flare along the wall. A second horn answers the first, much closer. Behind the
shut gate, boots move in numbers.
"#;

const CLOSURE_PROMPT: &str = r#"[ADDITIONAL INFORMATION] This ongoing act have {0} of at most {1} messages.
After writing this next beat normally, judge how close the ACT is to a natural close —
its goal from CURRENT ACT is your reference, not the mood of this conversation.

Then always end your reply with one line:

[CLOSURE]:<0-5>
(e.g., [CLOSURE]:2)

0   the act is in full motion; nothing points at its goal yet
1   the goal is far; the current thread needs real development
2   the goal is coming into view; real obstacles remain
3   the goal is nearly reached, or clearly out of reach — the story is arranging
    its endgame
4   the next beat could plausibly resolve the act — a confrontation, a discovery,
    a deal, a defeat
5   this beat itself resolves the act; the next beat should open a new act

Judge the fiction, not your own prose. A loud argument is not a close; a quiet
moment after it, where the story would naturally turn, is. Do not inflate the
number to be helpful — most beats are 1 or 2."#;

// TODO: Improve this prompt
const DISTILL_PROMPT: &str = r#"## DISTILL
Below you are given the existing memory (## CONTEXT: the current PAST ACTS, CURRENT
STATE, facts and memories) followed by the latest conversation turns. The existing
memory was true as of its last update; the conversation is what happened since.

Reconcile the two. Produce the memory record as it should stand NOW — not a record
of only the new turns, and not a copy of the old one. Return JSON only, with exactly
three fields:

- "facts": the facts that are true now (at the current moment of the adventure). For each one decide:
    - KEEP an existing fact unchanged if the conversation neither changed nor
      contradicted it, and it still matters.
    - OVERRIDE a fact when the conversation changes it: emit the new version with
      the same "subject", stating the new truth (e.g. a wound that healed, an ally
      who turned, a goal that was met or abandoned). The new fact replaces the old.
    - ADD a fact the conversation newly establishes.
    - DROP a "plot" fact once the conversation resolves it or makes it moot. Never
      drop a "permanent" fact.
  Each item is an object:
    - "subject": who or what the fact is about (a name or a thing), verbatim. Reuse
      the existing subject spelling so an updated fact lines up with the one it
      replaces.
    - "fact": one short, self-contained statement, in the present. Keep names,
      numbers and identifiers verbatim.
    - "permanence": one of
        "permanent"  true for the rest of the story — an identity, a bond, an
                     irreversible change. Never contradicted by later events.
        "plot"       true until the story moves on or says otherwise — a current
                     goal, a held grudge, an open deal.
  When a new fact clashes with an existing one, the conversation wins: the later
  truth replaces the earlier. Do not emit both the old and the new version of the
  same fact.

- "state": one object describing the single current situation AFTER these turns.
  Start from the existing CURRENT STATE and update only what the conversation moved;
  carry forward anything it did not touch. Do not reset a field to empty just
  because the latest turns did not mention it.
    - "day"                 in-world time now, e.g. "Day 2, dusk" (null if never established).
    - "place"               where the scene is now, e.g. "cave mouth" (unknown if not specified).
    - "characters_present"  names of everyone on scene now (only the speakers not npc interactions) — add arrivals, remove those who left.
    - "open_threads"        unresolved threads, each a short phrase. Drop a thread the conversation closed; add ones it opened.
    - "conditions"          active conditions now, each a short phrase, e.g. "kaelen: left arm, minor", "mood: tense", "storm approaching". Drop conditions that ended.

- "running_digest": a prose summary of the CURRENT ACT so far — the "THIS ACT SO FAR"
  section. This is a full rewrite, not an append: fold the existing THIS ACT SO FAR
  (if any) together with the new turns into one coherent, up-to-date account of what
  has happened in this act. A few tight paragraphs in past tense, naming who did what
  and how threads moved. It should read as the single story-so-far, not a changelog.
  Do not restate permanent lore or other acts; cover only the current act. If there
  is no prior digest, write it from the turns alone.

Record only what the existing memory or the conversation establishes. Never invent
anything. Prefer a few sharp facts over many vague ones. The "running_digest" is the
only prose; "facts" and "state" are structured. Return JSON only."#;

const SINGLE_CHARACTER_CHAT_PROMPT: &str = "You write one beat of an interactive roleplay. The turn belongs to one character, named below — continue it.";

const SINGLE_CHARACTER_CHAT_BODY_INSTRUCTION: &str = r#"## BODY
Do not restate the header's name in the prose. Begin with speech, with an action,
or with either. Any of these may appear in one turn, in any order:

"spoken words"              speech, heard by everyone present
*physical action*           action, third person, present tense
((private thought))         thought, heard by no one
(whisper to <name>) "..."   the quoted line is private to that character
"#;

const SINGLE_CHARACTER_CHAT_RULES_INSTRUCTION: &str = r#"## RULES
1. One voice per reply. Never write two characters in one turn.
2. Never write the player's words, thoughts, feelings or decisions — only how the
   world answers them.
3. Stay inside the voice of the character you chose: their wording, their rhythm,
   what they would never say. If no character would naturally respond, answer as
   [NARRATOR].
4. Two to four paragraphs. End where the player has something to answer/engage.
5. CURRENT STATE, the facts and the messages above are true. Never contradict them;
   a later fact beats an earlier one.
6. Do not resolve a conflict the player has not yet acted on.
7. Characters know only what they have witnessed or been told. Do not let one
   character act on another's private thought or whisper.
"#;

const SINGLE_CHARACTER_CHAT_EXAMPLES_INSTRUCTION: &str = r#"## EXAMPLES
Example — speech, then action
\"The gate won't open for us again. Not tonight, not with that horn still ringing.\"
*leans in the doorway, arms folded, waiting for someone to argue*

Example — action and thought, no speech
*sets the lantern down and kneels beside the wounded man, both hands on the wound*
((this is past herbs))

Example — aimed at someone
(lowers her voice, eyes on Kaelen) \"If the barkeep sold us once, he'll sell us again.
We leave before dawn.\"" },
"#;

const NARRATOR_CHAT_PROMPT: &str = "You are the narrator of an interactive roleplay. This beat belongs to the narrator: no character speaks.";

const NARRATOR_CHAT_BODY_INSTRUCTION: &str = r#"## BODY
Narration only. Third person, present tense. No quoted speech from any character
except the player's own last words if the world reacts to them. You may describe
what characters do and what becomes of it, but never decide an action for them.

Aim for one to three short paragraphs. Concrete, sensory, specific: what a person
present would notice. Withhold as much as you reveal — the world is not in a hurry.
"#;

const NARRATOR_CHAT_RULES_INSTRUCTION: &str = r#"## RULES
1. Never write the player's words, thoughts, feelings or decisions — only how the
   world moves around them.
2. Never resolve a conflict the player has not yet acted on. Escalate, complicate,
   or let it breathe — do not settle it.
3. CURRENT STATE, the facts and the messages above are true. Never contradict them;
   a later fact beats an earlier one.
4. Do not make a character present speak. If someone must answer, leave space for
   them: describe the moment before they would.
5. End where the player has something to answer/engage: a sound, an arrival, a silence
   that asks a question.
"#;

const NARRATOR_CHAT_EXAMPLES_INSTRUCTION: &str = r#"## EXAMPLES
Example — the world presses in
Torches flare along the wall. A second horn answers the first, much closer. Behind
the shut gate, boots move in numbers.

Example — an ending the party must answer
The ford is three hours behind them when the storm breaks. Rain turns the road to
clay, and ahead, at the treeline, a single rider watches without moving.

Example — time passing, tension held
Dawn is grey and slow. The fire burns down to embers; nobody has suggested sleep.
Somewhere down the slope, the horses stamp twice, then go quiet.
"#;

pub struct ContextBuilder;

impl ContextBuilder {
    pub async fn build_autonomous_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            AUTONOMOUS_CHAT_PROMPT.to_string(),
            ContextBuilder::build_settings_prompt(adventure),
            ContextBuilder::build_characters_prompt(adventure),
            AUTONOMOUS_CHAT_HEADER_INSTRUCTION.to_string(),
            AUTONOMOUS_CHAT_BODY_INSTRUCTION.to_string(),
            AUTONOMOUS_CHAT_RULES_INSTRUCTION.to_string(),
            AUTONOMOUS_CHAT_CHAT_EXAMPLES_INSTRUCTION.to_string(),
        ];

        ContextBuilder::assemble(conn, adventure, director, &head).await
    }

    pub async fn build_single_character_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
        character: &Character,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            SINGLE_CHARACTER_CHAT_PROMPT.to_string(),
            ContextBuilder::build_settings_prompt(adventure),
            SINGLE_CHARACTER_CHAT_BODY_INSTRUCTION.to_string(),
            SINGLE_CHARACTER_CHAT_RULES_INSTRUCTION.to_string(),
            SINGLE_CHARACTER_CHAT_EXAMPLES_INSTRUCTION.to_string(),
            character.render(),
        ];

        ContextBuilder::assemble(conn, adventure, director, &head).await
    }

    pub async fn build_narrator_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            NARRATOR_CHAT_PROMPT.to_string(),
            ContextBuilder::build_settings_prompt(adventure),
            NARRATOR_CHAT_BODY_INSTRUCTION.to_string(),
            NARRATOR_CHAT_RULES_INSTRUCTION.to_string(),
            NARRATOR_CHAT_EXAMPLES_INSTRUCTION.to_string(),
        ];

        ContextBuilder::assemble(conn, adventure, director, &head).await
    }

    async fn assemble(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
        head: &[String],
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let mut turns = vec![PromptTurn::system(head.join("\n\n"))];

        turns.push(PromptTurn::system(
            ContextBuilder::build_static_context(conn, adventure).await?,
        ));
        turns.extend(ContextBuilder::build_previous_messages(conn, adventure).await?);

        let messages_in_act = count_uncovered_per_act(conn, adventure.id).await? as u16;
        if director.should_request_closure(messages_in_act) {
            turns.push(PromptTurn::system(ContextBuilder::build_closure_prompt(
                messages_in_act,
                director.get_max_act_messages(),
            )));
        }

        Ok(turns)
    }

    pub fn build_settings_prompt(adventure: &Adventure) -> String {
        let mut lines = vec!["## SETTINGS".to_string()];

        lines.push(format!("Title: {}", adventure.title));
        if !adventure.description.trim().is_empty() {
            lines.push(format!("Premise: {}", adventure.description));
        }
        if let Some(genre) = adventure.genre.as_deref().filter(|g| !g.trim().is_empty()) {
            lines.push(format!("Genre: {genre}"));
        }
        if !adventure.tone.trim().is_empty() {
            lines.push(format!("Tone: {}", adventure.tone));
        }
        if !adventure.lore.text.trim().is_empty() {
            lines.push(format!("Lore: {}", adventure.lore.text));
        }
        lines.push(format!("Narrator: {}", adventure.narrator.name));

        lines.join("\n")
    }

    async fn build_static_context(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        let final_prompt = vec![
            "## CONTEXT".to_string(),
            ContextBuilder::build_acts_prompt(conn, adventure).await?,
            ContextBuilder::build_current_state(conn, adventure).await?,
            ContextBuilder::build_relevant_memories(conn, adventure).await?,
        ];

        Ok(final_prompt.join("\n\n"))
    }

    async fn build_acts_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        let acts = get_act(conn, adventure.id).await?;

        let current = acts.iter().find(|act| act.status == ActStatus::Active);
        let past: Vec<&Act> = acts
            .iter()
            .filter(|act| act.status != ActStatus::Active && act.digest.is_some())
            .collect();

        let mut sections = Vec::new();

        if !past.is_empty() {
            sections.push(format!("### PAST ACTS\n{}", Act::render_digest(&past)));
        }

        if let Some(act) = current {
            sections.push(format!("### CURRENT ACT\n{}", act.render_index()));

            let running = act.render_running_digest();
            if !running.trim().is_empty() {
                sections.push(format!("### THIS ACT SO FAR\n{running}"));
            }
        }

        Ok(sections.join("\n\n"))
    }

    async fn build_current_state(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        if let Some(state) = get_state(conn, adventure.id).await? {
            if let Some(rendered) = state.render() {
                return Ok(format!("### CURRENT STATE\n{0}", rendered).to_string());
            }
        }

        Ok(String::new())
    }

    /// The dynamic middle: the most recent messages, each a Player turn (the
    /// human) or a Cast turn (a character or narrator reply).
    async fn build_previous_messages(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let messages = fetch_last_n(conn, adventure.id, AUTONOMOUS_HISTORY_WINDOW).await?;

        Ok(messages
            .into_iter()
            .map(|message| ContextBuilder::message_to_turn(adventure, &message))
            .collect())
    }

    /// Classify a stored message. Does it come from the player or NPC?
    fn message_to_turn(adventure: &Adventure, message: &Message) -> PromptTurn {
        if ContextBuilder::is_player_message(adventure, message) {
            PromptTurn::player(message.text.clone())
        } else {
            PromptTurn::cast(format!("{}: {}", message.speaker_name, message.text))
        }
    }

    fn is_player_message(adventure: &Adventure, message: &Message) -> bool {
        if message.narrator_id.is_some() {
            return false;
        }

        match message.character_id {
            Some(author_id) => adventure
                .characters
                .iter()
                .any(|c| c.id == author_id && c.is_player),
            None => false,
        }
    }

    async fn build_relevant_memories(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        // TODO: retrieve the live facts for the acts
        Ok(String::new())
    }

    /// The full cast, each rendered in the adventure's tone.
    fn build_characters_prompt(adventure: &Adventure) -> String {
        if adventure.characters.is_empty() {
            return String::new();
        }

        let mut block = String::from("## CHARACTERS CAST");
        for character in &adventure.characters {
            block.push_str("\n");
            block.push_str(&character.render());
        }

        block
    }

    pub fn build_closure_prompt(
        current_number_of_messages: u16,
        current_act_messages_max_threshold: u16, // TODO: later it will be controlled on the adventure settings, for now defaults to 200
    ) -> String {
        CLOSURE_PROMPT
            .replace("{0}", &current_number_of_messages.to_string())
            .replace("{1}", &current_act_messages_max_threshold.to_string())
    }

    pub async fn build_distill_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        messages: &[Message],
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let mut turns = Vec::with_capacity(messages.len() + 2);
        turns.push(PromptTurn::system(DISTILL_PROMPT.to_string()));

        turns.push(PromptTurn::system(
            ContextBuilder::build_static_context(conn, adventure).await?,
        ));

        turns.extend(messages.iter().map(|message| {
            PromptTurn::player(format!("{}: {}", message.speaker_name, message.text))
        }));

        Ok(turns)
    }
}
