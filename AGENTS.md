# AGENTS.md

Local-first roleplay chat app. Rust binary, two front-ends (ratatui TUI + Slint GUI) over one
shared core, SQLite via `sqlx`, all LLMs local (Ollama / ComfyUI, OpenAI-compatible).

Read `docs/Flow.md` first for the wiring map, `docs/design-guide.md` for UI/terminology rules,
`README.md` for the product overview.

## Commands

Everything goes through `just` (root `justfile`, which imports `comfy-image.just`,
`comfy-video.just`, `comfy-audio.just` for model downloads and ComfyUI smoke tests).

| Command | What it does |
| --- | --- |
| `just build` | `cargo build` |
| `just test` | `cargo test` (offline-safe, see Testing) |
| `just test-ignored` | `cargo test --test mod -- --ignored --test-threads=1` — the LLM/ComfyUI tests; needs real servers, **must** stay single-threaded |
| `just lint` | **only** `cargo fmt`. There is no clippy target; don't claim one exists |
| `just run-debug` | deletes `*.db*` then runs the debug binary |
| `just watch` | `cargo watch -x run` |
| `just ui-preview BuilderPage src/ui/sl/builder/builder_page.slint` | live-reload a single Slint component |
| `just create-migration <name>` | `sqlx migrate add -r <name>` |
| `just db` / `just db-tables` / `just db-reset` | sqlite3 shell / list tables / delete `$DB` (default `living_voices.db`) |
| `just coverage` | `cargo llvm-cov`, ignores `src/ui/`, `src/cli/`, `app.rs` |
| `just provision-status` | what `./provision` already has, and the recipe to fix each gap |
| `just provision-llama` | fetch → extract → cmake build the engine into `provision/llama/build/bin/llama-server` |
| `just comfy-requirements` | venv + ComfyUI + torch into `provision/comfy_ui` |
| `just provision` | both of the above; the same steps the release job runs |
| `just provision-clean` | delete `./provision` |

- `.githooks/pre-commit` runs `cargo fmt`, `git add -u`, then `cargo test` — staged formatting
  changes land in the commit automatically.
- CI (`.github/workflows`) only runs `cargo test --locked` on pull requests.
- `build.rs` compiles `src/ui/sl/app.slint` into Rust; every `cargo build` regenerates the UI
  glue. Build-dependency `slint-build`, runtime `slint` with `renderer-software`.

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

- **The UI calls managers only** (`MessageManager`, `AdventureManager`, `CharacterManager`, …).
  It never touches `persistence` or `ai` directly.
- Domain has no dependencies on the other layers; persistence returns domain types; application
  orchestrates domain + persistence + ports.
- New LLM provider = new `impl` of an `application::port` trait in `src/ai/integrations/`. Trait
  methods return `impl Future<...> + Send` (RPITIT), not `#[async_trait]`.
- All prompt wording lives in `context/prompts.rs`; builders in `chat_context.rs` /
  `image_context.rs` only assemble consts plus character/message data. Never inline prompt text
  in managers or adapters.

## Conventions

- Edition 2024. Errors are `thiserror` enums per module (`CharacterManagerError`,
  `ContextBuilderError`, `PersistenceError`, `StateError`) with `#[from]` conversions.
- UI-facing copy and types say **adventure**, never "campaign" (`docs/design-guide.md`).
- Builders on domain types are consuming-chained: `Character::new(..).with_appearance(..)`
  (`None` by default, `with_*` sets `Some`).
- Slint: kebab-case properties in `.slint`, snake_case in Rust. Model exposure happens in
  `src/ui/viewmodels/mod.rs`; Slint callbacks are registered in `src/ui/callbacks/register.rs`
  and implemented one file per page.

## Gotchas

- **Managers must write twice.** Every mutation updates SQLite *and* `SharedState`, and inside
  the lock it must patch both `adventure.characters` and `player` when the id matches — see
  `create_character` / `update_character` in `src/backend/application/character_manager.rs`.
  Forgetting the `player` branch silently desyncs the UI.
- **Never hold a DB connection across an LLM `.await`.** `create_character_image_prompt` does an
  explicit `drop(conn)` before calling `prompt_handler.handle_prompt` and re-acquires after.
  SQLite pooling deadlocks otherwise.
- **Persistence uses runtime queries** (`sqlx::query(..).bind(..)`), not the `query!` macros, so
  the build needs no live database and no `.sqlx` cache.
- **Paths depend on the build profile** (`src/config.rs`): debug reads `config.toml` from the
  repo working dir, release from `$HOME/.living-voice`. `config-local.toml` is gitignored and
  copied from `config.toml` by `just dev-requirements`.
- `appearance` is prose and is fed to the **chat** prompt through `Character::render()`
  (`chat_context.rs`). The image side is three separate generated fragments, split by how
  often they change: `appearance_identity_prompt` (face/hair/build, never), `appearance_current_outfit`
  (per scene), `appearance_current_pose` (per beat). Never write clothing or framing into the
  identity prompt and never repeat the body in the outfit or pose prompts — the layers are
  concatenated by `ContextBuilder::compose_character_image_prompt`, so duplicates fight and the
  image model blends the contradiction.
- Image backend: `ImageModelWorkflow` (FluxQ4/FluxQ8/Qwen) selects a hard-coded ComfyUI JSON
  graph and injects the prompt at the `CLIPTextEncode` node. `ImageHandler` has one character
  entry point; framing (`ImageFraming::Portrait` vs `FullBody`) is applied when the prompt is
  composed, not by the adapter.

## UI data flow (do not re-read per edit)

`AppState` holds only the **ongoing adventure**: `current_adventure`, `current_act`,
`current_state`, `current_facts`, `player`. It is not a general-purpose cache — do not put
builder/authoring data in it. The writes that touch it (`load_adventure`, `update_adventure`,
`upsert_act`, `CharacterManager`) keep it current, so reading it costs no I/O.

The builder deliberately keeps nothing: `BuilderPageContext` (`ui/callbacks/builder_page.rs`)
holds no data at all and reads what the view needs on render.

- **`open(adventure_id)`** is the one expensive read: `load_adventure` (the 8-way join) warms the
  snapshot, then the first `show`.
- **`show(section)`** renders from two sources: the adventure from the snapshot (free), the acts
  and "is there an opening" from two small indexed queries (`get_acts`,
  `get_messages(adventure_id, 1)`). That is the whole cost of a render — never re-run
  `get_adventure` per save.
- **Saves write once, then `show(None)`.** The re-render is what keeps `StorySection`'s `dirty`
  indicator honest.
- Slint never mutates navigation state itself: the sidebar raises `section-requested`, the
  character page raises `builder-entered` (`app.slint`), and Rust renders *then* sets `section`
  / `current-page`. Do not reintroduce a `<=>` binding on `BuilderGlobals.section`.
- A new character is created immediately via `CharacterManager::create_draft`, which persists
  `Character::draft(adventure_id)` and returns it so the page never reads it back. Its name is
  `Character::PLACEHOLDER_NAME` until the player types one, and `builder_view` does not count a
  placeholder as a filled cast (`is_placeholder`).

## Testing

- Tests are integration-style under `tests/`, mirroring the `src/` tree, all pulled in from
  `tests/mod.rs` (`mod backend; mod ai;`). Unit tests inside `src/` are rare.
- `tests/backend/application/helper.rs` and `tests/backend/persistence/helper.rs` are the shared
  fixtures: `setup_db()` creates a real temp SQLite file named
  `living_voices_memory_test_<uuid>.db` in the crate root, runs migrations, and returns a
  `TempDb` guard that deletes the file (plus `-wal`/`-shm`) on drop. Note `run_migrations`
  consumes its pool, so `setup_db` opens a second pool for the test to use. Copy that pattern
  instead of inventing one.
- Anything needing a live model is `#[ignore = "requires a running Ollama server"]` (or
  "requires a running ComfyUI server …"), so plain `cargo test` needs no servers. Those tests hit
  the real `http://localhost:11434/v1` / `:8188` and share DB files, which is why
  `just test-ignored` pins `--test-threads=1`.
- Tests are commented `// given` / `// when` / `// then` blocks — keep that shape.

## Working preference: teach, don't hand over

**Default mode is teaching.** Unless the user explicitly asks for finished code ("just write it",
"give me the code", "do it for me", "stop guiding"), do not dump complete implementations. Explain
the approach, name the parts and how they connect, point at where the code goes and what it must
do, offer hints or a sketch, then let the user write it and review what they wrote.

Explicit hand-over requests that switch the mode off: "just write it", "give me the code", "do it
for me", "stop guiding", "implement it".

Boilerplate, scaffolding, config, and **tests** are fine to write outright — the learning value is
in the feature logic, not in the glue. `.kiro/skills/guide-me/SKILL.md` (gitignored, local) holds
the long form of this mode.

### Where the user is strong and where not

- **Slint UI: inexperienced.** They understand the idea (declarative components, `.slint` + Rust
  glue) but do not have the syntax at their fingertips. Always explain Slint work step by step:
  what each property/callback does, how the Rust side binds to it (`viewmodels/mod.rs`,
  `callbacks/register.rs`), and why kebab-case in `.slint` becomes snake_case in Rust. Prefer
  showing a small component in isolation, and remember `just ui-preview <Component> <file>` for a
  live window. Do not assume they can read a Slint layout at a glance.
- **Rust: comfortable.** No need to explain ownership basics, `Result`, or traits. But when a
  change touches performance, memory, or concurrency, surface the tradeoff and say why the chosen
  shape is cheaper — stack vs heap, clone vs borrow, `Arc`/`RwLock` contention, holding a lock or a
  pooled connection across an `.await` (see the gotchas above), N+1 queries, needless `String`
  allocation. Only when it actually fits the change; no unsolicited lectures.

Everything else: default to teaching, stay concise, and when a request is ambiguous (e.g. "can
you improve my code" with no "just do it"), teach the shape of the fix instead of silently
rewriting their files. Reviewing and pointing out what breaks beats replacing their code.
