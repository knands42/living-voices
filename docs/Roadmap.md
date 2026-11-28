# Roadmap

## In flight

- [ ] Distribute an adventure — see [Adventure catalog](#adventure-catalog-sharing)

## Later

### Adventure catalog (sharing)

Goal: a catalog is data, not infrastructure. One document format and one manifest; the app reads any
base URL, so hosting is swappable and self-hosting later is just another URL.

- [ ] Export/import one adventure as a document: `schema_version`, cast, lore, acts, narrator,
      `art { full, thumb, sha256 }`, license.
- [ ] Hub reads `index.json` once (ETag kept in memory); Start clones that adventure's document + art.
- [ ] Static hosting only: public bucket, content-addressed immutable URLs, no accounts, no tracking,
- [ ] If possible, add a tyni woker in front of the CDN to reject requests without the header parameter `LivingVoices: <token>`
- [ ] On the CDN side, add cache-control to avoid request abusing (and rate limit)
- [ ] Sign index and documents (Ed25519); the app rejects unsigned or altered entries.
- [ ] Host-agnostic: R2, B2 + Cloudflare, Scaleway, Hetzner, Bunny, or a self-hosted bucket.

### Media generation (ComfyUI)

| model         | use                             | ComfyUI support               | notes                                                               |
| ------------- | ------------------------------- | ----------------------------- | ------------------------------------------------------------------- |
| Wan 2.1       | video (3s loop)                 | native                        | heaviest / slowest on 8GB; on-command only                          |
| MMAudio       | video sound                     | custom node                   | takes a video as input                                              |
| F5-TTS        | character speech (first choice) | custom node (TTS-Audio-Suite) | TTS: text -> spoken voice; lighter, main env, voice cloning         |
| IndexTTS-2    | character speech (fallback)     | custom node (TTS-Audio-Suite) | richer emotion, but cannot offload properly, needs more VRAM        |
| ACE-Step 15xl | background music (optional)     | native                        | music gen: empty lyrics = instrumental; lyrics = sung, not dialogue |

### Character AI

Goal: keep every character in voice. The corpus comes first and costs no training; weights are the
last step, for protagonists only.

**Step 1: corpus (every character)**

1. Sheet is the seed: `name`, `headline`, `backstory`, `traits`, `message_example`.
2. Derive a voice spec from the sheet with the strongest local model, one per character.
3. Self-play N scenes x M turns per character, seeded only by the sheet.
4. Judge every sample against the sheet (voice match, fact consistency, register); keep the good,
   drop the rest. A UI thumbs-down turns the same pairs into chosen/rejected.
5. Use the accepted corpus as few-shot exemplars, and retrieve past scenes, in the live prompt.

**Step 2: adapter (1-3 protagonists, only if they still drift)**

6. QLoRA on that corpus, ship the adapter as GGUF, serve it as a derived Ollama model, score it on
   held-out scenes before it replaces the prompt. `llama-server` switches adapters server-wide
   (`/lora-adapters`) and resets the prompt cache, so route at scene granularity, never per turn.
   Train the voice conditioned on the character sheet so live state (body, outfit, pose) stays
   authoritative, and gate the adapter on reply-format adherence — a strong one breaks the JSON.

| technology                                     | what it does                                                       |
| ---------------------------------------------- | ------------------------------------------------------------------ |
| Ollama `/v1` (already wired)                   | bulk-generate voice specs and scenes offline with the strong model |
| judge prompt in `context/prompts.rs`           | second pass scoring a sample against the sheet, discarding drift   |
| few-shot `PromptTurn`s                         | feed accepted samples into `ChatPort`, no trait change needed      |
| `nomic-embed-text` / `bge-m3` via `/api/embed` | turn past scenes into vectors for retrieval                        |
| `sqlite-vec` (or cosine over a BLOB)           | store and query those vectors next to the rest of the DB           |
| unsloth / llama-factory                        | QLoRA fine-tune of the persona corpus                              |
| `convert_lora_to_gguf.py` + Ollama `ADAPTER`   | ship the adapter as a per-character derived model                  |
| held-out scenes + judge score                  | the before/after number that decides if an adapter ships           |

Lands as `CharacterCorpus` (domain), `character_samples` (persistence), `persona_manager.rs`
(application), `persona_port.rs` + `ai/integrations/text/persona.rs`. Generation runs in
`Task::perform`/`Task::run`, and no DB connection may be held across the model `.await`.

### Character image identity (LoRA + references)

Goal: identity lives in the weights, state lives in the prompt. V1 stays deterministic: one prompt
mode and one reference rule per mode, no dynamic reference selection.

- [ ] Character carries an image adapter (`path`, `scale`, base-model fingerprint); a missing or
      mismatched adapter degrades to reference-only with a warning.
- [ ] Dataset contract: 5 captioned blocks (4 outfits + nude, 20 each), the same poses across every
      block, plus expression shots; caption pose, outfit, expression and setting, and caption the
      nude state so it is not the identity default.
- [ ] Prompt: adapter present → describe only the delta (pose, outfit, scene); adapter absent →
      the full appearance description the builder already sends.
- [ ] Reference: adapter present → text-to-image, no reference; adapter absent → the newest full
      body, today's chain. Exact outfits opt into an outfit reference later.
- [ ] Validate on a held-out outfit, pose and expression before an adapter is accepted.
- [ ] Not in V1: pose control, dynamic reference picking, per-scene references.

### Adventure tooling

Goal: an adventure can carry its own panels — a calendar, an inbox, a ledger — without the app
knowing about them. Text stays the substrate; a tool is a view over state the turn produced.

Two problems, two machines, never merged:

- **Authored artifacts** — opening frame, newspaper clippings, calendar card. Written or generated
  once at adventure creation, cached, rendered as a prop instead of chat prose. No state.
- **Live tools** — state that mutates per turn. Manifest + append-only event log + pure reducers.

Architecture: **the model proposes, the application owns state.** Proposals ride the existing
structured tail of the reply (`ChatReply::act_closure` is the precedent), so no second model call per
turn and no reliance on native function-calling, which small local models do not do reliably. The
reducer rejects illegal commands — advancing three days without an act boundary — and logs the
rejection instead of dropping it silently. State is a projection replayed from events, so a payload
change costs a replay, not a migration.

| piece               | where                                            |
| ------------------- | ------------------------------------------------ |
| `AdventureTool`     | `domain/tool.rs`, beside `lore.rs`               |
| `ToolCommand`       | domain; the only things a model may ask for      |
| `ToolReducer`       | domain, pure, no I/O                             |
| `extract_proposals` | `application/managers/message/tool_pass.rs`      |
| manifest + events   | `persistence/tool_store.rs`                      |
| panels              | `ui/pages/chat/panels/<kind>.rs`, routed by slot |

Prompt fragments per tool live in `context/prompts.rs` and are assembled by the context builder; a
tool with no fragment does not exist to the model, so an adventure with no tools keeps today's
prompt byte for byte.

- [ ] Phase 1 — the smallest honest slice: `ToolKind::Calendar`, one `AdvanceDays` command, a sidebar
      panel, and the fragment that lets the narrator advance a day on a scene close. Proves the seam.
- [ ] Split the pass: `extract_proposals` (parser, degrades to zero proposals on malformed output,
      never fails the turn) and `apply_proposals` (reducer loop), both pure and testable.
- [ ] Tables: `adventure_tool`, `tool_event` (carries `message_id`, so the transcript can say when a
      panel changed), `tool_state` as projection. Rebuild tool state on open; never guess old rows.
- [ ] Builder step for authoring tools, beside lore and cast, with declarative `enabled_when` over act
      state — a phone exists from act 1, the calendar appears when act 2 opens. Data, not code.
- [ ] Second kind — `Inbox`, where a tool needs prose: one `PromptPort` call composed from
      `context/prompts.rs`, run through `Task::perform` on the same turn's `Task`.
- [ ] Authored artifacts: one `IntroFrame` type (text blocks + optional generated image), created
      once, cached, rendered as a card. This is the cheap version of the CraveUai effect and covers
      most of it.
- [ ] Explicitly not building: a per-turn stage-manager agent that owns tool state. It cannot hold
      the day of the week across turns and doubles latency on a card ComfyUI also wants.

- Gallery / memory: showcase all media generated per act, like an album.
- Narrator settings.

**Engine / providers**

- Benchmark default vs vllm setup: script maybe.
- CPU embedding for character selection.

**Gameplay**

- Challenges & achievements: score on quest completion.
- Final per-journey prompt about the score: end-of-journey summary.
