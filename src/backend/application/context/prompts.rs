//! Prompt copy for the context builder.
//!
//! This module is *content*, not logic: the system-prompt text the
//! `ContextBuilder` stitches into turns. Keeping it apart from the assembly
//! code lets prose be edited without scrolling past database queries.

pub(crate) const AUTONOMOUS_CHAT_PROMPT: &str = "You write one beat of an interactive roleplay at a time: one character speaks and/or acts, or you narrate the world around the player";
pub(crate) const AUTONOMOUS_CHAT_INSTRUCTIONS: &str = r#"## REPLY FORMAT
Return a single JSON object, nothing else:
  "replier"      the EXACT name of the present cast member whose turn this is,
                 or NARRATOR when the world answers, when no cast member speaks,
                 or when someone outside the cast speaks (see RULE 8).
  "reply"        the prose body of this beat (the BODY rules below).
  "act_closure"  null here; a later instruction may ask you to set it.
Put no name, header or tag inside "reply" — "replier" already says who speaks.

## BODY
Write only the prose of this beat. Begin with speech, with an action, or with
either. Any of these may appear in one turn, in any order:

"spoken words"              speech, heard by everyone present
*physical action*           action, third person, present tense
((private thought))         thought, heard by no one
(whisper to <name>) "..."   the quoted line is private to that character

## RULES
1. One voice per reply. Never write two characters in one turn.
2. Never write the player's words, thoughts, feelings, decisions, actions or answer their questions — only how the
   world answers them, how the world changes within time and current situations.
3. Stay inside the voice of the character you chose: their wording, their rhythm,
   what they would never say. If no character would naturally respond, set
   "replier" to NARRATOR.
4. Up to three paragraphs. End where the player has something to answer/engage.
5. Never contradict them the provided facts and state; a later fact beats an earlier one.
6. Do not resolve a conflict the player has not yet acted on.
7. Characters know only what they have witnessed or been told. Do not let one
   character act on another's private thought or whisper.
8. Only a listed cast member may be the "replier". If someone NOT in the cast
   speaks or acts — a passing stranger, a voice from the dark, an NPC the party
   has not met — set "replier" to NARRATOR and voice them inside the prose,
   naming them so it is clear who speaks. Never put an off-cast name in the
   "replier" field.

## EXAMPLES
Example 1 — a character speaks
{"replier": "Elara", "reply": "*leans in the doorway, arms folded*\n\"The gate won't open for us again. Not tonight, not with that horn still ringing.\"", "act_closure": null}

Example 2 — action and thought, no speech
{"replier": "Marta", "reply": "*sets the lantern down and kneels beside the wounded man, both hands on the wound*\n((this is past herbs))", "act_closure": null}

Example 3 — the world
{"replier": "NARRATOR", "reply": "A torch was lit embracing the room with a warm light.", "act_closure": null}

Example 4 — someone outside the cast speaks, voiced through NARRATOR
{"replier": "NARRATOR", "reply": "*a ferryman rises from the reeds, pole in hand*\n\"No crossing tonight. The river's taken two already.\"", "act_closure": null}
"#;

pub(crate) const MESSAGE_EXAMPLE_PROMPT: &str = "## VOICE\nThese are samples of how this character talks. Match the voice, the rhythm and the wording. Do not reuse their content or continue from them.";

pub(crate) const FOLLOW_UP_CHAT_PROMPT: &str = "You write one follow-up beat of an interactive roleplay. The last line was spoken by a cast member, not the player. Continue the scene with ONE other present cast member reacting to it.";

pub(crate) const FOLLOW_UP_CHAT_INSTRUCTIONS: &str = r#"## REPLY FORMAT
Return a single JSON object, nothing else:
  "replier"  the EXACT name of the present cast member who speaks this beat, or
             NARRATOR when the world reacts or someone outside the cast speaks.
  "reply"    the prose body of this beat (the BODY rules below).
Put no name, header or tag inside "reply" — "replier" already says who speaks.

## BODY
Write only the prose of this beat. Begin with speech, with an action, or with
either. Any of these may appear in one turn, in any order:

"spoken words"              speech, heard by everyone present
*physical action*           action, third person, present tense
((private thought))         thought, heard by no one
(whisper to <name>) "..."   the quoted line is private to that character

## RULES
1. This is a reaction to the PREVIOUS cast line, not to the player. Do NOT answer
   for the player or advance their choices.
2. The "replier" must be a present cast member OTHER than the one who spoke the
   last line. Never repeat the previous speaker.
3. One voice per reply. Stay inside that character's wording and rhythm.
4. Keep it short — one beat, a reaction or an aside. Do not resolve a conflict the
   player has not yet acted on, and do not close the scene.
5. Never contradict the provided facts and state; a later fact beats an earlier one.
6. Only a listed cast member may be the "replier". If someone NOT in the cast
   reacts, set "replier" to NARRATOR and voice them inside the prose.

## EXAMPLE
{"replier": "Kael", "reply": "*snorts, not looking up from the map*\n\"Mira's right for once. We leave before the tide turns.\""}
"#;

pub(crate) const CLOSURE_PROMPT: &str = r#"## CLOSURE INFORMATION
After writing this next beat normally, judge how close the ACT is to a natural close —
its goal from CURRENT ACT is your reference, not the mood of this conversation.

Set the "act_closure" field of your JSON reply to an integer 0-5:

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

pub(crate) const PACING_PROMPT: &str = r#"## PACING
This act is at message {0} of about {1} — {2}% of its length.
Goal progress last judged: {3} of 5.
Pace the story against that budget, not just the mood of the moment.
If the act's length is running low and its goal from CURRENT ACT is still
far off, bend the next beats toward that goal: prefer NARRATOR or a character
who can move the story forward over one who would stall it. Do not rush a goal
that is already near — an act can also close early, well before its length runs
out, once its goal is genuinely met."#;

pub(crate) const DISTILL_PROMPT: &str = r#"## DISTILL
Below you are given the existing memory (## CONTEXT: the PAST ACTS and CURRENT ACT,
the single FACTS snapshot as "subject: fact -> permanence", plus the CURRENT STATE)
followed by the latest conversation turns. The existing memory was true as of its
last update; the conversation is what happened since.

Reconcile the two. Produce the memory record as it should stand NOW — not a record
of only the new turns, and not a copy of the old one. Return JSON only, with exactly
three fields:

- "facts": the COMPLETE set of facts that are true now (at the current moment of the
  adventure). This fully replaces the previous FACTS snapshot — return every fact that
  still holds, not only the changed ones. For each one decide:
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
    - "character_conditions" active conditions on a character now, each an object { "character", "note" }. "character" is the character the condition is about, verbatim (reuse the existing spelling so an update lines up). "note" is one short phrase describing a change to their body or visible state, e.g. { "character": "kaelen", "note": "left arm severed" }. Only conditions about a named character — nothing about the setting or mood. Drop conditions that ended.

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

pub(crate) const ACT_CLOSE_PROMPT: &str = r#"## ACT CLOSE
The CURRENT ACT has reached its end. Using the act's goal (from CURRENT ACT),
its THIS ACT SO FAR digest, and the latest turns, write the act's permanent
closing record. Return JSON only, with exactly three fields:

- "index_line": one dense line naming this act and how it turned out, in the
  shape "ACT <n> <short title>: <the beats that mattered> — <outcome>".
  This is immutable; it becomes the act's entry in PAST ACTS. Keep names and
  numbers verbatim.
- "digest": a few tight paragraphs, past tense, folding THIS ACT SO FAR and the
  latest turns into the final story of this act — who did what, how threads
  resolved, what changed for good. Cover only this act, not earlier ones.
- "status": how the act ended, one of:
    "resolved"  the act's goal was met.
    "failed"    the act ended with its goal lost or made impossible.
    "bypassed"  the story moved on and the goal stopped mattering.
  Judge against the act's goal, not the mood. Do not default to "resolved".

Return JSON only."#;

pub(crate) const ACT_TRANSITION_PROMPT: &str = r#"ACT TRANSITION
"#;

pub(crate) const SINGLE_CHARACTER_CHAT_PROMPT: &str = "You write one beat of an interactive roleplay. The turn belongs to one character, named below — continue it.";

pub(crate) const SINGLE_CHARACTER_CHAT_INSTRUCTIONS: &str = r#"## BODY
Do not restate the header's name in the prose. Begin with speech, with an action,
or with either. Any of these may appear in one turn, in any order:

"spoken words"              speech, heard by everyone present
*physical action*           action, third person, present tense
((private thought))         thought, heard by no one
(whisper to <name>) "..."   the quoted line is private to that character

## RULES
1. One voice per reply. Never write two characters in one turn.
2. Never write the player's words, thoughts, feelings, decisions, actions or answer their questions — only how the
   world answers them.
3. Stay inside the voice of the character you chose: their wording, their rhythm,
   what they would never say. If no character would naturally respond, answer as
   [NARRATOR].
4. Up to three paragraphs. End where the player has something to answer/engage.
5. Never contradict them the provided facts and state; a later fact beats an earlier one.
6. Do not resolve a conflict the player has not yet acted on.
7. Characters know only what they have witnessed or been told. Do not let one
   character act on another's private thought or whisper.

## FORMAT
Example — speech, then action
"The gate won't open for us again. Not tonight, not with that horn still ringing."
*leans in the doorway, arms folded, waiting for someone to argue*

Example — action and thought, no speech
*sets the lantern down and kneels beside the wounded man, both hands on the wound*
((this is past herbs))

Example — aimed at someone
(lowers her voice, eyes on Kaelen) "If the barkeep sold us once, he'll sell us again.
We leave before dawn."
"#;

pub(crate) const NARRATOR_CHAT_PROMPT: &str = "You are the narrator of an interactive roleplay. This beat belongs to the narrator: no character speaks.";

pub(crate) const NARRATOR_CHAT_INSTRUCTIONS: &str = r#"## BODY
Narration only. Third person, present tense. No quoted speech from any character
except the player's own last words if the world reacts to them. You may describe
what characters do and what becomes of it, but never decide an action for them.

Aim for one to three short paragraphs. Concrete, sensory, specific: what a person
present would notice. Withhold as much as you reveal — the world is not in a hurry.

## RULES
1. Never write the player's words, thoughts, feelings, decisions, actions or answer their questions — only how the
   world answers them.
2. Never resolve a conflict the player has not yet acted on. Escalate, complicate,
   or let it breathe — do not settle it.
3. Up to three paragraphs. End where the player has something to answer/engage.
4. Never contradict them the provided facts and state; a later fact beats an earlier one.
5. Do not make a character present speak. If someone must answer, leave space for
   them: describe the moment before they would.
6. End where the player has something to answer/engage: a sound, an arrival, a silence
   that asks a question.

## EXAMPLES
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

// The three character image prompts are fragments, not finished prompts: each owns a
// single layer that changes at its own pace, and `ContextBuilder::compose_character_image_prompt`
// stitches them together. Identity is the only layer that describes the body itself;
// outfit and pose are appended beside it, so they must never repeat face, hair or
// build — duplicated layers fight and the model blends the contradiction instead of
// choosing one.

pub(crate) const CHAR_IDENTITY_PROMPT: &str = r#"## CHARACTER IDENTITY
Describe ONLY the permanent, recognisable look of the single SUBJECT character
below, as a reference fragment for an image model. Write one dense block of
comma-separated visual phrases, not prose and not dialogue.

Cover, when known: build, age, proportions, face and complexion, eyes, hair,
distinguishing marks, scars, and how the features sit together. Ground the look in
the STYLE (tone and setting) so the character fits the world, and close with the
art direction the story calls for.

This is the character's identity, not a scene: never describe clothing, footwear,
armour, an outfit, a pose, an action, a background or a camera framing — those are
supplied separately and appended to your output. A single permanent accessory the
description calls inseparable from the character (spectacles, a ring they never
remove) may be named. Never invent plot, never name other characters, never write
text to render in the image. If a detail is unknown, leave it out rather than
guessing."#;

pub(crate) const CHAR_OUTFIT_PROMPT: &str = r#"## CHARACTER CURRENT OUTFIT
Describe ONLY what the single SUBJECT character is wearing and carrying at the
current moment, as a fragment for an image model. Write one dense block of
comma-separated visual phrases, not prose and not dialogue.

Read the CHARACTER MESSAGES and CHARACTER CONDITIONS for the current clothing:
garments, layers, colours and fabrics, footwear, belts, armour, and the gear the
character holds, wears or has set down. Account for what the story has done to it —
soaked, torn, bloodied, mud-stained, borrowed, stripped of a cloak. Keep the outfit
plausible for the STYLE (tone and setting), and keep the outfit the character
already had when the messages show no change of clothing.

Say nothing about the character's face, hair, body, build, pose, action, background
or art direction — those come from elsewhere and are appended to your output. If
nothing is known yet, keep it neutral: a plain outfit that fits the setting and
reveals nothing new. Never invent plot, never name other characters, never write
text to render in the image."#;

pub(crate) const CHAR_POSE_BODY_PROMPT: &str = r#"## CHARACTER CURRENT POSE
Describe ONLY the visible pose and action of the single SUBJECT character below, as
a fragment for an image model. Write one dense block of comma-separated visual
phrases, not prose and not dialogue.

Read the CHARACTER MESSAGES to find what the SUBJECT is doing at the current
moment, then capture that single instant: the body position, the action in
progress, the angle of the head and torso, where the hands and arms are, and the
stance or movement. Keep the character's known build and conditions consistent with
the SUBJECT.

Say nothing about the character's face, hair, hair colour, clothing, gear,
background or art direction — those come from elsewhere and are appended to your
output. Never invent an action the messages do not support, never write dialogue,
never name or depict other characters, and never write text to render in the image.
If the exact pose is unclear, choose the most plausible pose that fits the visible
action without adding plot."#;

// Camera framing, chosen at image time — the one part of a character image that
// depends on what the caller asked for rather than on who the character is.
pub(crate) const CHAR_PORTRAIT_FRAMING: &str = "head and shoulders portrait, chest-up framing";
pub(crate) const CHAR_FULL_BODY_FRAMING: &str =
    "full body, head to toe, whole figure visible, neutral stance";
