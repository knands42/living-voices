use sqlx::SqliteConnection;

use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::message::Message;
use crate::backend::persistence::error::PersistenceError;

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

const DISTILL_PROMPT: &str = r#"## DISTILL
Below you are given the existing memory (## CONTEXT: the current PAST ACTS, CURRENT
STATE, facts and memories) followed by the latest conversation turns. The existing
memory was true as of its last update; the conversation is what happened since.

Reconcile the two. Produce the memory record as it should stand NOW — not a record
of only the new turns, and not a copy of the old one. Return JSON only, with exactly
two fields:

- "facts": the facts that are true now. For each one decide:
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

Record only what the existing memory or the conversation establishes. Never invent
anything. Prefer a few sharp facts over many vague ones. Do not write prose —
JSON only."#;

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
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            AUTONOMOUS_CHAT_PROMPT.to_string(),
            ContextBuilder::build_settings_prompt(conn).await?,
            ContextBuilder::build_characters_prompt(conn).await?,
            AUTONOMOUS_CHAT_HEADER_INSTRUCTION.to_string(),
            AUTONOMOUS_CHAT_BODY_INSTRUCTION.to_string(),
            AUTONOMOUS_CHAT_RULES_INSTRUCTION.to_string(),
            AUTONOMOUS_CHAT_CHAT_EXAMPLES_INSTRUCTION.to_string(),
        ];

        ContextBuilder::assemble(conn, &head).await
    }

    pub async fn build_single_character_chat_prompt(
        conn: &mut SqliteConnection,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            SINGLE_CHARACTER_CHAT_PROMPT.to_string(),
            ContextBuilder::build_settings_prompt(conn).await?,
            SINGLE_CHARACTER_CHAT_BODY_INSTRUCTION.to_string(),
            SINGLE_CHARACTER_CHAT_RULES_INSTRUCTION.to_string(),
            SINGLE_CHARACTER_CHAT_EXAMPLES_INSTRUCTION.to_string(),
            ContextBuilder::build_character_prompt(conn).await?,
        ];

        ContextBuilder::assemble(conn, &head).await
    }

    pub async fn build_narrator_chat_prompt(
        conn: &mut SqliteConnection,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            NARRATOR_CHAT_PROMPT.to_string(),
            ContextBuilder::build_settings_prompt(conn).await?,
            NARRATOR_CHAT_BODY_INSTRUCTION.to_string(),
            NARRATOR_CHAT_RULES_INSTRUCTION.to_string(),
            NARRATOR_CHAT_EXAMPLES_INSTRUCTION.to_string(),
        ];

        ContextBuilder::assemble(conn, &head).await
    }

    async fn assemble(
        conn: &mut SqliteConnection,
        head: &[String],
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let mut turns = vec![PromptTurn::system(head.join("\n\n"))];

        turns.push(PromptTurn::system(
            ContextBuilder::build_static_context(conn).await?,
        ));
        turns.extend(ContextBuilder::build_previous_messages(conn).await?);

        // TODO: on demand, not every call
        turns.push(PromptTurn::system(ContextBuilder::build_closure_prompt(
            10, 200,
        )));

        Ok(turns)
    }

    pub async fn build_settings_prompt(
        conn: &mut SqliteConnection,
    ) -> Result<String, PersistenceError> {
        todo!()
    }

    async fn build_static_context(conn: &mut SqliteConnection) -> Result<String, PersistenceError> {
        let final_prompt = vec![
            "## CONTEXT".to_string(),
            ContextBuilder::build_acts_prompt(conn).await?,
            ContextBuilder::build_current_state(conn).await?,
            ContextBuilder::build_relevant_memories(conn).await?,
        ];

        Ok(final_prompt.join("\n\n"))
    }

    async fn build_acts_prompt(conn: &mut SqliteConnection) -> Result<String, PersistenceError> {
        let mut final_prompt = String::new();

        final_prompt.push_str("PAST ACTS\n");
        // TODO: retrieve from db here all previous acts.digest
        final_prompt.push_str("CURRENT ACT\n");
        // TODO: retrieve from db the current active act.index
        final_prompt.push_str("THIS ACT SO FAR\n");
        // TODO: retrieve from db the current active act.running_digest

        Ok(final_prompt)
    }

    async fn build_current_state(conn: &mut SqliteConnection) -> Result<String, PersistenceError> {
        let mut final_prompt = String::new();

        final_prompt.push_str("CURRENT STATE\n");
        // TODO: retrieve from db the current state
        final_prompt.push_str("Time: \n");
        final_prompt.push_str("Place: \n");
        final_prompt.push_str("Present: \n");
        final_prompt.push_str("Open Threads: \n");
        final_prompt.push_str("Conditions: \n");

        Ok(final_prompt)
    }

    /// The dynamic middle: each past message as a Player turn (the human) or a
    /// Cast turn (a character or narrator reply).
    async fn build_previous_messages(
        conn: &mut SqliteConnection,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        todo!()
    }

    async fn build_relevant_memories(
        conn: &mut SqliteConnection,
    ) -> Result<String, PersistenceError> {
        todo!()
    }

    async fn build_characters_prompt(
        conn: &mut SqliteConnection,
    ) -> Result<String, PersistenceError> {
        // for each character retrieved build_character_prompt()
        todo!()
    }

    async fn build_character_prompt(
        conn: &mut SqliteConnection,
    ) -> Result<String, PersistenceError> {
        todo!()
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
        messages: &[Message],
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let mut turns = Vec::with_capacity(messages.len() + 1);
        turns.push(PromptTurn::system(DISTILL_PROMPT.to_string()));

        turns.push(PromptTurn::system(
            ContextBuilder::build_static_context(conn).await?,
        ));
        turns.extend(ContextBuilder::build_previous_messages(conn).await?);

        turns.extend(messages.iter().map(|message| {
            PromptTurn::player(format!("{}: {}", message.speaker_name, message.text))
        }));

        Ok(turns)
    }
}
