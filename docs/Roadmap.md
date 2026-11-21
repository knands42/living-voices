# Roadmap

## Now

### Migrate Slint to iced

Done at the shell level: `src/ui/main.rs` boots iced, `src/ui/app.rs` routes, pages live in
`src/ui/pages/` and own their boot/update/view. Native async (`Task`/`Message`) and better text
support were the deciding factors. Remaining: the pages still living only in `src/ui/slint_bkp/`.

### In flight

- [ ] ...

## Later

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

### Features

**UI**

- Gallery / memory: showcase all media generated per act, like an album.
- Narrator settings.

**Engine / providers**

- Benchmark default vs vllm setup: script maybe.
- CPU embedding for character selection.

**Gameplay**

- Challenges & achievements: score on quest completion.
- Final per-journey prompt about the score: end-of-journey summary.
