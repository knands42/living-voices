# Application Flow

High-level map of how the app is wired. Read this first.

```mermaid
flowchart TD
    UI["UI<br/>iced (GUI) · ratatui (TUI)"]

    subgraph Managers["Managers (the only thing the UI calls)"]
        MM["MessageManager"]
        AM["AdventureManager"]
    end

    State["SharedState<br/>single source of truth<br/>Arc + RwLock"]
    DB[("SQLite<br/>(sqlx)")]
    LLM["Local LLMs<br/>chat · narrator · memory · image"]

    UI --> MM
    UI --> AM

    MM --> State
    AM --> State

    MM --> DB
    AM --> DB

    MM --> LLM
```

## How a turn flows

```mermaid
sequenceDiagram
    participant UI
    participant MM as MessageManager
    participant S as SharedState
    participant DB as SQLite
    participant LLM

    UI->>MM: send_message(text)
    MM->>DB: store message
    MM->>LLM: ask who speaks + reply
    LLM-->>MM: reply
    MM->>DB: store reply
    Note over MM,LLM: if the act must close:<br/>narration + autonomous prompt + background distill
    MM->>S: refresh current act / state / facts
    MM-->>UI: reply
```

## Notes

- The UI calls **only** the managers. Managers are the one layer exposed as modules to the UI.
- Managers own the `SharedState` behind an `Arc` + `RwLock`: one source of truth, many readers, one writer at a time.
