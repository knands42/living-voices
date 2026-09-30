# Living Voices

A local-first roleplay chat app built in **Rust + Slint**. Create characters and a world, then talk with them — with a Narrator that steers events in the background.

Not a novel editor. Not a VTT. The chat is the product.

## What it does

- **Roleplay chat** — solo or group conversations with AI characters, each with a persistent persona (voice, goals, what they'll reveal) and memory.
- **Narrator** — a third actor that watches the conversation and interrupts with events when the story needs a push ("a guard rounds the corner…").
- **Actions, not just words** — your input is typed: **Say**, **Do**, **Think** (private, NPCs never know), **Whisper** (one NPC hears it).
- **Scene images** — a local image model occasionally renders the current situation, giving a visual heartbeat to the story.
- **Time & world** — a story clock advances; the world ticks and NPCs remember what happened, append-only, so they never "forget".

## The multi-model pipeline

Everything runs locally. Each LLM has one job, driven by background processing jobs:

| Job | Model | Responsibility |
| --- | --- | --- |
| **Chat** | main LLM | Generates character responses from a prepared summary of the conversation + character personas/memory (full recent lines when not yet compressed). |
| **Narrator** | small LLM | Watches the flow; decides *when* to interrupt and generates the event. |
| **Memory** | small LLM | Extracts events/facts into an append-only memory store; compresses older chat context without losing what matters. |
| **Image** | local image model | Generates scene snapshots at narrative beats. |

A job queue (background workers) orchestrates these so the chat LLM only ever consumes clean, prepared context — never the raw unbounded transcript.

## Architecture

```
┌──────────────────────────────┐
│  Slint UI (Rust frontend)    │
│  chat · characters · world   │
└──────────────┬───────────────┘
               │ messages/events (async channels)
┌──────────────┴───────────────┐
│  Core (Rust)                 │
│  session state · story clock │
│  event store (append-only)   │
└──────┬───────┬───────┬───────┘
       │       │       │
┌──────┴─┐ ┌───┴───┐ ┌─┴────────┐
│ Chat   │ │Memory │ │ Narrator │   ← background jobs
│ job    │ │ job   │ │ job      │
└──────┬─┘ └───┬───┘ └─┬────────┘
       │       │       │
┌──────┴───────┴───────┴────────┐
│ Local LLM runtime (llama.cpp/ │
│ Ollama) + image model         │
└───────────────────────────────┘
```

- **Frontend:** Slint (declarative UI in Rust), dark warm theme.
- **Core:** owns sessions, the story clock, and the append-only event log — every message, action, and memory is an event (this makes rewind/branching cheap later).
- **Jobs:** async workers, each bound to one model, communicating through the core's event bus.
- **Memory:** append-only records on entities; context compression promotes recent events to summaries, never deletes.

## Getting started

```bash
# prerequisites: Rust (stable), a local LLM runtime (Ollama or llama.cpp server)
cargo run
```

1. Create a character (name + short description → persona) or load one from a world.
2. Start a conversation — solo, or a group scene.
3. Talk with `Say`, act with `Do`, keep secrets with `Think`, share one-on-one with `Whisper`.

## Roadmap

- [ ] MVP: solo + group chat, Say/Do/Think/Whisper grammar
- [ ] Narrator with interrupt events
- [ ] Memory extraction + context compression pipeline
- [ ] Scene snapshot generation (image job)
- [ ] Story clock + time advance
- [ ] Rewind / what-if branches (event-sourced)

## Non-goals

No long-form novel editing, no RPG rules engine, no cloud dependency — the chat is the product and it runs on your machine.

## AI usage in this project

No vibecoding, any bug this project has, it's entirely my fault. AI is used deliberately for:

- Getting unstuck on specific implementations (still learning Rust)
- Troubleshooting (tons of UI wiring and state management)
- Writing tests (with my review)
- Complex layouts (SLint is a new UI library for me)
- Initial UI mockups — a starting point

## Status

Early stage. Rust + Slint, design system and view specs live in [`docs/`](docs/).
