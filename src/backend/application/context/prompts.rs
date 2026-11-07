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
Put no name, header or tag inside "reply": never start it with "NARRATOR:",
"Name:", "[CHARACTER]" or any other label. "replier" already says who speaks.

## BODY
Write only the prose of this beat. Begin with speech, with an action, or with
either. Any of these may appear in one turn, in any order:

"spoken words"              speech, heard by everyone present
*physical action*           action, third person, present tense
((private thought))         thought, heard by no one
(whisper to <name>) "..."   the quoted line is private to that character

The action and the thought are the replier's own. Another person may appear, but
the only voice in this reply is the replier's.

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
8. "replier" is a present cast member or NARRATOR, and nothing else. Use
   NARRATOR when the world answers, when no cast member would speak, or when
   someone outside the cast acts — a passing stranger, a voice from the dark, an
   NPC the party has not met. Voice those inside the prose and let "replier" say
   who owns the beat. Never put an off-cast name in the "replier" field.

## EXAMPLES
A cast member, with speech, action and private thought:
{"replier": "Elara", "reply": "*leans in the doorway, arms folded*\n\"The gate won't open for us again. Not tonight, not with that horn still ringing.\"\n((two nights, at most))", "act_closure": null}

The world answers, here with someone outside the cast speaking:
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
   last line, or NARRATOR when the world reacts. Never repeat the previous
   speaker.
3. One voice per reply. Stay inside that character's wording and rhythm.
4. Keep it short — one beat, a reaction or an aside. Do not resolve a conflict the
   player has not yet acted on, and do not close the scene.
5. Never contradict the provided facts and state; a later fact beats an earlier one.
6. "replier" is a present cast member or NARRATOR, and nothing else. When
   someone outside the cast reacts, set "replier" to NARRATOR and voice them
   inside the prose with no label in front of it.

## EXAMPLE
{"replier": "Kael", "reply": "*snorts, not looking up from the map*\n\"Mira's right for once. We leave before the tide turns.\""}
"#;

pub(crate) const CLOSURE_PROMPT: &str = r#"## CLOSURE INFORMATION
Judge this beat against the goal in CURRENT ACT, then set the "act_closure" field to an
integer: 0 nothing points at it yet, 2 obstacles remain, 3 the endgame is forming,
4 the next beat could resolve it, 5 this beat resolves it.
Judge the goal, not the mood: a loud moment is not a close. Most beats are 1 or 2."#;

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
You are given the memory as it stood (## CONTEXT: PAST ACTS, CURRENT ACT, FACTS as
"subject: fact -> permanence", CURRENT STATE) and the turns since. The memory was
true at its last update; the turns are what happened after.

Return JSON only, with exactly three fields, describing the memory as it stands NOW —
not only the new turns, and not a copy of the old one.

- "facts": the complete replacement set of facts true at this moment. KEEP an
  unchanged fact that still matters, OVERRIDE a changed one under the same "subject"
  (a wound that healed, an ally who turned), ADD what the turns newly establish, DROP
  a "plot" fact once it is resolved or moot. Never drop a "permanent" one. The turns
  beat the memory: emit one version of a fact, never both. Each item is
  { "subject": who or what it is about, verbatim and spelled as before so an update
    lines up, "fact": one short present-tense statement keeping names and numbers
    verbatim, "permanence": "permanent" (an identity, a bond, an irreversible change)
    or "plot" (true until the story moves on) }.

- "state": the single situation after these turns. Start from CURRENT STATE, change
  only what moved, carry the rest; never empty a field because the turns were silent.
  Fields: "day" (in-world time, null if never established), "place", "characters_present"
  (everyone on scene now, by name), "open_threads" (unresolved, short phrases),
  "character_conditions" (each { "character", "note" } for a named character's body or
  visible state; nothing about mood or setting).

- "running_digest": rewrite THIS ACT SO FAR by folding the existing digest and the new
  turns into one account of this act. Past tense, a few tight paragraphs, who did what
  and how threads moved. Only this act: no changelog, no lore, no earlier acts.

Record only what the memory or the turns establish. Never invent. Prefer a few sharp
facts over many vague ones."#;

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
