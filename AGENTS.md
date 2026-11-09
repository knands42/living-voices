# AGENTS.md

Local-first roleplay chat app. Rust binary, two front-ends (ratatui TUI + Slint GUI) over one
shared core, SQLite via `sqlx`, all LLMs local (Ollama / ComfyUI, OpenAI-compatible). Read
`docs/Flow.md` for the wiring map, `docs/design-guide.md` for UI/terminology rules, `README.md`
for the product overview.

## Commands

Everything goes through `just` (root `justfile`, importing `comfy-*.just` for model downloads and
ComfyUI smoke tests).

| Command | What it does |
| --- | --- |
| `just build` | `cargo build` |
| `just test` | `cargo test` (offline-safe, see Testing) |
| `just test-ignored` | `cargo test --test mod -- --ignored --test-threads=1` — LLM/ComfyUI tests; need real servers, **must** stay single-threaded |
| `just lint` | **only** `cargo fmt`. There is no clippy target; don't claim one exists |
| `just run-debug` | deletes `*.db*` then runs the debug binary |
| `just watch` | `cargo watch -x run` |
| `just ui-preview BuilderPage src/ui/sl/builder/builder_page.slint` | live-reload one Slint component |
| `just create-migration <name>` | `sqlx migrate add -r <name>` |
| `just db` / `just db-tables` / `just db-reset` | sqlite3 shell / list tables / delete `$DB` (default `living_voices.db`) |
| `just coverage` | `cargo llvm-cov`, ignoring `src/ui/`, `src/cli/`, `app.rs` |
| `just provision-status` | what `./llama`, `./comfyui`, `./provision` have, and the recipe for each gap |
| `just provision-llama` | fetch → extract → cmake build → install `llama/llama-server` |
| `just provision-llama-model` | download the GGUF into `llama/models` (same bytes fetched on first run) |
| `just provision-comfy` | pinned ComfyUI + `comfyui/python` + venv from `comfy-requirements.lock` inside `comfyui/`; models stay in `*-model-all` |
| `just provision-comfy-lock` | re-freeze the lock and attach a wheel sha256 per pin (`uv pip compile --generate-hashes`); installs use `--require-hashes`, so bump it when a version/commit/index moves |
| `just provision-comfy-bundle` | release artifact `provision/comfyui-<version>.tar.zst` + `.sha256` (no `.git`, no venv, no models) |
| `just provision` / `provision-clean` | all of the above / delete `./provision`, `./llama`, `./comfyui` |

- `.githooks/pre-commit` runs `cargo fmt`, `git add -u`, then `cargo test` — staged formatting
  changes land in the commit.
- CI only runs `cargo test --locked` on pull requests.
- `build.rs` compiles `src/ui/sl/app.slint` into Rust; every `cargo build` regenerates the glue
  (`slint-build` build-dep, `slint` runtime with `renderer-software`).

## Layout

```
src/main.rs, lib.rs, app.rs     binary entry, lib root, app bootstrap
src/backend/domain/             pure types, no I/O (no sqlx, no reqwest)
src/backend/application/        managers + prompt builders + traits
  port/                         ChatHandler / PromptHandler / ImageHandler / MemoryHandler
  context/prompts.rs            every LLM system prompt, as &str consts
  state.rs                      SharedState = Arc<RwLock<AppState>>
src/backend/persistence/        one file per aggregate, raw sqlx queries
src/ai/integrations/            adapters: text/{chat,memory,narrator,open_api_client}, image/comfy_image
src/ui/                         Slint: sl/*.slint + callbacks/ + viewmodels/ + window.rs
src/cli/                        ratatui TUI: draw/ + events/
tests/                          mirrors src/, single binary entry tests/mod.rs
```

Layering rules the code actually follows:

- **The UI calls managers only** (`MessageManager`, `AdventureManager`, `CharacterManager`, …);
  never `persistence` or `ai` directly.
- Domain depends on no other layer; persistence returns domain types; application orchestrates
  domain + persistence + ports.
- New LLM provider = new `impl` of an `application::port` trait in `src/ai/integrations/`. Trait
  methods return `impl Future<...> + Send` (RPITIT), not `#[async_trait]`.
- All prompt wording lives in `context/prompts.rs`; builders in `chat_context.rs` /
  `image_context.rs` only assemble consts plus character/message data. Never inline prompt text in
  managers or adapters.

## Conventions

- Edition 2024. Errors are `thiserror` enums per module (`CharacterManagerError`,
  `ContextBuilderError`, `PersistenceError`, `StateError`) with `#[from]` conversions.
- UI-facing copy and types say **adventure**, never "campaign" (`docs/design-guide.md`).
- Domain builders are consuming-chained: `Character::new(..).with_appearance(..)` (`None` by
  default, `with_*` sets `Some`).
- Slint: kebab-case properties in `.slint`, snake_case in Rust. Models are exposed in
  `src/ui/viewmodels/mod.rs`; callbacks registered in `src/ui/callbacks/register.rs` and
  implemented one file per page.

## Code style: the code is the comment

**Prefer code that needs none.** A comment is a failure of naming before it is a courtesy: if a
line needs a note to say what it does, rename the variable, extract the helper, or flatten the
conditional until the note is redundant. `///` doc comments are no exception — avoid them for the
same reason, they are noise. This applies to the agent as much as to the user.

A comment is earned only by a **why** the code cannot carry, and then it stays short: an external
constraint (SQLite semantics, llama.cpp flags, an applied migration, a provider's HTTP shape), a
workaround with what breaks without it, or an invariant a reader could break while editing. Never
restate the next line, narrate a body step by step, keep dead code "for reference", or leave a
`TODO` where an issue belongs. Delete the comment and read the code again — if the meaning
survived, it was noise.

The one standing exception is the `// given` / `// when` / `// then` shape required in tests (see
Testing).

## Gotchas

- **Managers must write twice.** Every mutation updates SQLite *and* `SharedState`, patching both
  `adventure.characters` and `player` when the id matches — see `create_character` /
  `update_character` in `character_manager.rs`. Missing the `player` branch desyncs the UI silently.
- **Never hold a DB connection across an LLM `.await`.** `create_character_image_prompt` does
  `drop(conn)` before `handle_prompt` and re-acquires after; SQLite pooling deadlocks otherwise.
- **Persistence uses runtime queries** (`sqlx::query(..).bind(..)`), not the `query!` macros, so the
  build needs no live database and no `.sqlx` cache.
- **Paths depend on the build profile** (`src/config/mod.rs`): debug reads `config.toml` from the repo
  working dir, release from `$HOME/.living-voice`. `config-local.toml` is gitignored, copied from
  `config.toml` by `just dev-requirements`. The first-run gate writes its one answer back into
  `config.toml` (`[models.image] enabled` + `[setup] media_engine_asked`); `--setup` asks again.
- **Three appearance layers, each owned by one field.** `appearance` is the authored identity, already
  shaped as an image fragment (face, hair, build, marks, art direction) — only the original full body
  is drawn from it, and `Character::render()` feeds `current_look()` to the chat prompt. The layers
  that age during play are `appearance_current_body` (`None` while the story has not changed the body,
  so `current_look()` falls back to `appearance`, which is why it can never go stale),
  `appearance_current_outfit` (never empty: the author supplies it, played scenes rewrite it) and
  `appearance_current_pose` (per beat). Never put clothing in the body layer or body in outfit/pose —
  `ContextBuilder::compose_character_image_prompt` concatenates them, so duplicates fight and the
  model blends the contradiction. A pose shot carries **no** framing clause: the pose decides the
  frame.
- **Image backend**: the graphs live in `ai/integrations/image/workflow.rs` and `Workflow` builds
  them, so `comfy_image.rs` only posts what it is given. Two config slots decide which:
  `workflow` draws originals (text-to-image), `reference_workflow` draws every shot that starts from
  a reference. With a Kontext model in the second slot the reference is injected as `ReferenceLatent`
  conditioning and the framing sizes an empty latent, so the prompt decides the crop; with anything
  else it falls back to img2img, where the reference *is* the latent and its framing survives. Node
  ids are found by `class_type`, never assumed. Prompt wording is applied by
  `ContextBuilder::compose_character_image_prompt`; only the shot's dimensions belong to the graph.
- **Every shot after the first is drawn against the newest full body.** `latest_full_body` generates
  the full body when the character has none, which is why a character's first image is always one.
  A reference reaches the adapter as `ShotReference { path, framing }`; ComfyUI's `LoadImage` only
  reads its workspace `input` dir, so the adapter copies the shot there first.
  Denoise belongs to the workflow (`Workflow::denoise`): Flux 0.6, Qwen 0.75, Kontext 1.0.
- **Shots are numbered, never overwritten**: `{adventure}/{character}/{character}_{shot}_{n}`, the
  first of a framing `_0`. ComfyUI appends its own `_00001_` counter after that, so the number is the
  one `CharacterGallery` wrote — parse it, don't compare filenames as text.

## UI data flow (do not re-read per edit)

`AppState` holds only the **ongoing adventure**: `current_adventure`, `current_act`,
`current_state`, `current_facts`, `player`. Not a general-purpose cache — no builder/authoring data.
`load_adventure`, `update_adventure`, `upsert_act` and `CharacterManager` keep it current, so
reading it costs no I/O. The builder keeps nothing: `BuilderPageContext`
(`ui/callbacks/builder_page.rs`) holds no data and reads what the view needs on render.

- **`open(adventure_id)`** is the one expensive read: `load_adventure` (the 8-way join) warms the
  snapshot, then the first `show`.
- **`show(section)`** renders the adventure from the snapshot (free) plus the acts and "is there an
  opening" from two small indexed queries (`get_acts`, `get_messages(id, 1)`). That is the whole
  cost of a render — never re-run `get_adventure` per save.
- **Saves write once, then `show(None)`**; the re-render is what keeps `StorySection`'s `dirty`
  indicator honest.
- Slint never mutates navigation state: the sidebar raises `section-requested`, the character page
  raises `builder-entered` (`app.slint`), and Rust renders *then* sets `section` / `current-page`. Do
  not reintroduce a `<=>` binding on `BuilderGlobals.section`.
- A new character is created immediately by `CharacterManager::create_draft`, which persists
  `Character::draft(adventure_id)` and returns it so the page never reads it back. Its name is
  `Character::PLACEHOLDER_NAME` until the player types one, and `builder_view` does not count a
  placeholder as a filled cast (`is_placeholder`).

## Testing

- Integration-style under `tests/`, mirroring `src/`, all pulled in from `tests/mod.rs`
  (`mod backend; mod ai;`). Unit tests inside `src/` are rare.
- `tests/backend/application/helper.rs` and `tests/backend/persistence/helper.rs` are the shared
  fixtures: `setup_db()` creates a real temp SQLite file
  (`living_voices_memory_test_<uuid>.db` in the crate root), runs migrations, and returns a `TempDb`
  guard that deletes it (plus `-wal`/`-shm`) on drop. `run_migrations` consumes its pool, so
  `setup_db` opens a second one for the test. Copy that pattern, don't invent one.
- Anything needing a live model is `#[ignore = "requires a running Ollama server"]` (or ComfyUI), so
  plain `cargo test` needs no servers. Those hit real `:11434/v1` / `:8188` and share DB files, hence
  `--test-threads=1`.
- Tests are commented `// given` / `// when` / `// then` blocks — keep that shape.
- Extend an existing test rather than adding one per field: a test should read as one clear story in
  the IDE tree, so prefer fewer, broader, readable tests over a new test for every new entry.

## Working preference: teach, don't hand over

**Default mode is teaching.** Unless the user explicitly asks for finished code ("just write it",
"give me the code", "do it for me", "stop guiding", "implement it"), do not dump complete
implementations: explain the approach, name the parts and how they connect, point at where the code
goes and what it must do, then let the user write it and review what they wrote. Boilerplate,
scaffolding, config and **tests** are fine to write outright — the learning value is in the feature
logic, not the glue. `.kiro/skills/guide-me/SKILL.md` (gitignored, local) holds the long form.

### Teaching shape: pick the architecture, show the flow

They are already a developer. Abstract prose about "separating concerns" teaches nothing; a concrete
architecture they can implement teaches everything. Do not describe the idea — **choose the structure
and trace the flow through it**, using the real types and call paths:

1. **Name the architecture and why** (one or two sentences, with the constraint that forces it).
2. **The parts**, as they would appear in code: structs, traits, functions, signatures.
3. **The flow**, entry point to effect, naming which piece owns what.
4. **Where the code goes** (file, module, layer) and what each piece must guarantee.

The register expected:

- `SharedState`: not "share data between the UI and managers" but *"a single `Arc<RwLock<AppState>>`,
  because cloning the `Arc` gives every holder a handle to the same allocation: the UI holds a
  manager, the manager holds the shared state, and a mutation through any path is visible to all —
  the lock makes that sharing safe, the clone is cheap."*
- A manual llama.cpp build pipeline: not "write a build script" but *"a `Step` struct holding the
  command plus its failure policy, and `run_pipeline(steps: &[Step], on_event: impl Fn(Progress))`
  iterating the steps and calling back per update — the callback is the seam that lets the CLI print
  and the UI render without the pipeline knowing either exists."*

Rule of thumb: if the answer could be pasted into a README without losing anything, it is too
abstract. Give the skeleton they can type in, plus why each seam exists.

### Where the user is strong and where not

- **Slint UI: inexperienced.** They know the idea (declarative components, `.slint` + Rust glue) but
  not the syntax. Explain step by step: what each property/callback does, how Rust binds to it
  (`viewmodels/mod.rs`, `callbacks/register.rs`), and why kebab-case in `.slint` becomes snake_case
  in Rust. Prefer a small component in isolation, and remember
  `just ui-preview <Component> <file>`. Do not assume they can read a layout at a glance.
- **Rust: comfortable.** No need to explain ownership, `Result` or traits. But when a change touches
  performance, memory or concurrency, surface the tradeoff and why the shape is cheaper — stack vs
  heap, clone vs borrow, `Arc`/`RwLock` contention, a lock or pooled connection held across an
  `.await` (see gotchas), N+1 queries, needless `String`. Only when it fits the change.

Otherwise: default to teaching, stay concise, and when a request is ambiguous (e.g. "can you
improve my code" with no "just do it"), teach the shape of the fix instead of rewriting their files.
Reviewing what breaks beats replacing their code.
