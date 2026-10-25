# Living Voices

A local-first roleplay chat app built in **Rust**, with a terminal UI (ratatui) and a desktop UI (Slint). Create characters and a world, then talk with them — with a Narrator that steers events in the background.

Not a novel editor. Not a VTT. The chat is the product.

## What it does

- **Roleplay chat** — solo or group conversations with AI characters, each with a persistent persona (voice, goals, what they'll reveal) and memory.
- **Narrator** — a third actor that watches the conversation and interrupts with events when the story needs a push ("a guard rounds the corner…").
- **Scene images** — a local image model occasionally renders the current situation, giving a visual heartbeat to the story.
- **Time & world** — a story clock and location changes.

## Architecture

Everything runs locally. The app is a Rust binary with two front-ends over a
shared core, and SQLite for persistence.

```
┌───────────────────────────────┐
│  UI                           │
│  ratatui (TUI) · Slint (GUI)  │
└───────────────┬───────────────┘
                │ state + messages
┌───────────────┴───────────────┐
│  Core (Rust)                  │
│  adventures · characters ·    │
│  messages · memory            │
└──────┬─────────────────┬──────┘
       │                 │
┌──────┴──────┐   ┌──────┴──────┐
│  SQLite     │   │ Local LLMs  │
│ (sqlx)      │   │ (Ollama) +  │
│ adventures, │   │ image model │
│ characters, │   └─────────────┘
│ messages,   │
│ memory      │
└─────────────┘
```

Each model has one job:

| Job | Model | Responsibility |
| --- | --- | --- |
| **Chat** | main LLM | Generates character responses from a character's persona/memory plus the recent conversation. |
| **Narrator** | main LLM | Watches the flow and decides *when* to interrupt with an event. |
| **Memory** | small LLM | Extracts facts into an append-only memory store and compresses older context so the chat LLM only sees clean, prepared context. |
| **Image** | local image model | Generates character portraits / scene snapshots. |

## Getting started

```bash
# prerequisites: Rust (stable), a local LLM runtime (Ollama or llama.cpp server)
cargo run
```

1. Create a character (name and short description → persona), or load one from a world.
2. Start a conversation — solo, or a group scene.
3. Talk using standard TTRPG message formatting:

| Write | Means | Who sees it |
| --- | --- | --- |
| `"spoken words"` | speech | everyone present |
| `*physical action*` | action (third person, present tense) | everyone present |
| `((private thought))` | thought | no one |
| `(whisper to <name>) "..."` | whisper | only that character |

## Non-goals

No long-form novel editing, no RPG rules engine, no cloud dependency. The chat is the final goal, and it runs on your machine only.
Maybe later, mobile support, but no plans.

## AI usage in this project

No vibecoding, any bug this project has, it's entirely my fault. AI is used deliberately for:

- Getting unstuck on specific implementations (still learning Rust)
- Troubleshooting (tons of UI wiring and state management on both SLintUI and RatatUI)
- Writing/Improving tests
- UI layouts
- Boilerplate

## Status

Early stage with some [`docs`](docs/) available.