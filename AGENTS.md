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
  (`chat_context.rs`), while `appearance_portrait_prompt` / `appearance_full_body_prompt` /
  `appearance_current_pose` are machine prompts produced by `CharacterManager` and consumed by
  `ImageHandler`. They are not interchangeable; see the note below.
- Image backend: `ImageModelWorkflow` (FluxQ4/FluxQ8/Qwen) selects a hard-coded ComfyUI JSON
  graph and injects the prompt at the `CLIPTextEncode` node; framing is decided by the caller via
  `generate_character_portrait` vs `generate_character_image`.

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

## Working preference

`.kiro/skills/guide-me/SKILL.md` (gitignored, local) defines a teaching mode: on "let me take
control", "guide me", "teach me", the user writes the core logic and agents supply scaffolding,
tests, and explanation only. Switch back to handing over code if they ask for it explicitly.
