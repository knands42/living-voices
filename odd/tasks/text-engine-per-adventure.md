# Text engine per adventure

Move the text engine's lifetime from app boot to adventure open, so its adapter set can be
per adventure. Provisioning (model downloads) stays at setup; only spawning moves.

## Specs

**S1.** The request, verbatim:

> on my @docs/Roadmap.md I need to implement this -> Load the text engine per adventure
>
> Basically each adventure will have itś own config (later They will have per character cast
> training, thats why, but I want to adapt it now, including starting the text engine with the new
> lora paramenter(s) when available).... I believe I first should stop calling the engines ready
> start from the provision pipeline

**S2.** The roadmap section, verbatim:

> ### Load the text engine per adventure
>
> Today `start_text_engine` spawns `chat` and `memory` at boot. The adapter set is per adventure,
> so the chat server's lifetime should be too — not the whole app's.
>
> - [ ] Endpoints move out of `Config` into `SharedState`: ports are searched at spawn, so the base
>       URL cannot be stamped once at boot.
> - [ ] Split the lifetimes: `chat` follows the open adventure, `memory`/distiller stays app-wide.
> - [ ] Characters carry `adapter_path`, `adapter_scale` and a base-model fingerprint (migration).
>       A missing or mismatched adapter degrades to no adapter with a warning, and never blocks
>       opening the adventure.
> - [ ] `spawn` receives the adventure's adapters (`--lora`, `--lora-init-without-apply`); narrator
>       beats post all scales to `0`.
> - [ ] Opening an adventure shows a loading state while `wait_until_ready` polls, and a switch away
>       mid-spawn abandons the pending child instead of racing it.
> - [ ] Deferred until the 60 GB box can be measured: pool keyed by (base model, adapter set), N warm,
>       evict LRU. Not before real load times are known.

**S3.** Model download (`provision_text_engine` → `ensure_chat` / `ensure_memory`) stays in the
setup pipeline. Only `start_text_engine`'s process spawn moves.

**S4.** The distiller (`memory`) keeps app-wide lifetime; only `chat` follows the adventure.

**S5.** Adapters are spawn-time input: `--lora` per character that has one,
`--lora-init-without-apply`. A missing or mismatched adapter degrades to no adapter with a
warning and never blocks opening the adventure.

**S6.** Narrator beats run with every adapter scale at `0`; a character beat sets its own scale to
`adapter_scale` and the rest to `0`.

**S7.** `EngineProfile::chat()` `ctx: 8192` and `EngineProfile::memory()` `gpu_layers: "2"` are 8 GiB
compromises; revisit when the target hardware lands, not before.

**S8.** Config flow comes first. The request, verbatim:

> Requesting start_engines requires nothing, an adventure will only hold the loras parameters (and
> others that might seem fit per adventrure) and call start_engines with it... I do not need to know
> the endpoint, start engines can just call config and override the config.local.... We just need to
> first architect the flow of config... Config will have a default initial state, but because it can
> change, either we re-read the config.local file every time we need a config or config become an
> Arc<> so when the app is first executed start_engines will have a way to override the existing
> config that is being used also the the AI integration implementations

**S9.** Config is read once and shared, never re-read per request: the spawned port is chosen by
`get_available_port`, so it cannot be persisted to `config.local`, and re-reading would put disk I/O
on the request path and race between adventures. A live read guard must never be held across an
`.await`.

**S10.** Pages never see an endpoint. `start_engines` writes the spawned URLs into the shared config
and every holder sees them; an adventure supplies only its adapter parameters.

## Tasks

- [x] T1a — `ConfigHandle` (`Arc<RwLock<Config>>`) in `src/config/mod.rs` with narrow accessors that
      copy and drop the guard, plus `set_text_endpoints`. Additive: no call site changed. (S8, S9)
- [x] T1b — `ChatHandler`, `NarratorHandler`, `MemoryHandler` hold `ConfigHandle` and read the live
      endpoint in `client()`. (S8, S9, S10)
- [x] T1c — Thread `ConfigHandle` through `app.rs`, `main.rs`, `ui/main.rs`, `ui/app.rs`,
      `ui/pages/mod.rs`, `ui/pages/setup/mod.rs` and `RunningEngines::stamp`. (S10)
- [x] T2 — `start_engines` takes the handle plus the adventure's adapter parameters and writes the
      spawned URLs into it. (S1, S10)
- [x] T3 — Split boot: `provision_text_engine` (downloads) and ComfyUI stay in setup; the chat spawn
      leaves it. (S3, S4)
- [x] T4 — The chat engine spawns when an adventure opens, in `Task::perform`, with a loading state
      and abandonment of a pending spawn on switch. (S2)
- [ ] T5 — Adapter columns + migration on characters, with base-model fingerprint and soft failure.
      (S5)
- [ ] T6 — `LlamaServer::spawn` takes the adapter list; narrator beats post all scales to `0`.
      (S5, S6)

## Log

- L1 — The user's request, quoted verbatim as S1.
- L2 — Key finding: facts persist as one JSON blob per adventure (`fact_persistence.rs:44`), but acts
      are one row per act (`create_memory.up.sql:3`), so `upsert` replaces facts and cannot delete
      acts. Engine lifecycle work does not touch this.
- L3 — Boot flow evidence: `setup/mod.rs:165-175` stamps the config from `RunningEngines`; the
      chat/memory URLs come from `entrypoint.rs:145` `start_text_engine`; `temp`/`start_provisioning`
      (`entrypoint.rs:49`) only downloads models.
- L4 — Config always reaches the pages as `Arc<Config>` already (`pages/mod.rs:28`); it goes stale
      because handlers copy one `TextModel` section at boot (`app.rs:81-84`,
      `chat_handler.rs:65`). `ChatHandler::client()` rebuilds the HTTP client per request from
      `self.config.base_url`, so a shared handle makes the endpoint live without touching pages.
- L5 — The user redirects the fork: config flow first, `Arc` not re-read, `start_engines` writes the
      URLs into it and takes only adapter parameters. Recorded as S8, S9, S10.
- L6 — The writer executor returned `quota` twice (T1 and the earlier meta-rail unit). T1 was split:
      T1a shipped inline (a small module change), T1b/T1c wait for a working executor.
- L7 — T1b and T1c shipped inline. `ActClose` took the handle too. Tests build handles through
      `tests/support.rs` (`chat_config` / `memory_config`) rather than a production test hook.
- L8 — OPEN: `ConfigHandle::set_text_endpoints` persists to `config.toml` (the user's own edit). The
      spawned port is searched, so a later launch starts on a dead endpoint instead of falling back
      to Ollama. `set_media_endpoint` deliberately does not persist. Needs a decision.
- L9 — OPEN: `MemoryHandler` derives `Deserialize` with `#[serde(skip)]` on its config field, so
      `ConfigHandle: Default` had to be implemented. It fabricates a code-default config that is
      never used for I/O. The real fix is separating the deserialize target from the handler.
- L10 — T2/T3/T4 shipped. `start_engines(config, loras)` stamps the handle. The chat page spawns
      `start_chat_engine(&adventure.loras)` inside its `open` task and keeps the child in page state;
      a bumped token discards a late result, dropping it kills the child. Memory spawns once on the
      first chat open and lives in `App`. Spawned URLs go through `set_chat_endpoint` /
      `set_memory_endpoint`, which do not persist, and are published only when the result is adopted.
- L11 — OPEN: leaving chat kills the engine but the live config keeps its port, so a failed respawn
      falls back to a dead port instead of Ollama. Ends when endpoints leave `Config` for
      `SharedState` (the roadmap's first unchecked item).
