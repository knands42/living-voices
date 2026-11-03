# Roadmap

## Media generation (via ComfyUI)

| model         | use                             | ComfyUI support               | notes                                                               |
|---------------|---------------------------------|-------------------------------|---------------------------------------------------------------------|
| Wan 2.1       | video (3s loop)                 | native                        | heaviest / slowest on 8GB; on-command only                          |
| MMAudio       | video sound                     | custom node                   | takes a video as input                                              |
| F5-TTS        | character speech (first choice) | custom node (TTS-Audio-Suite) | TTS: text -> spoken voice; lighter, main env, voice cloning         |
| IndexTTS-2    | character speech (fallback)     | custom node (TTS-Audio-Suite) | richer emotion, but cannot offload properly, needs more VRAM        |
| ACE-Step 15xl | background music (optional)     | native                        | music gen: empty lyrics = instrumental; lyrics = sung, not dialogue |

## Features

| feature                                  | notes                                               |
|------------------------------------------|-----------------------------------------------------|
| Gallery / Memory                         | showcase all media generated per act, like an album |
| Narrator settings                        |                                                     |
| Benchmark default vs vllm setup          | script maybe                                        |
| CPU embedding for character selection    |                                                     |
| Challenges & achievements (quest score)  | score on quest completion                           |
| Final per-journey prompt about the score | end-of-journey summary                              |

## TODOs (tracked here until moved to GitHub issues)

Priority order. P1 = do first, P3 = nice to have.

| # | pri | where | what |
|---|-----|-------|------|
| 1 | P1 | `message_manager.rs` | Make the flow easier to read. Keep cutting unneeded SQL queries (some already done). |
| 2 | P1 | `message_manager.rs` | Add benchmarks. Worst case: an active act closes, which fires a narration message + an autonomous chat prompt + a background distill (2 LLM calls, lots of Rust). Never measured. |
| 3 | P2 | `message_manager.rs:send_message` | New act starts with 0 messages. When that happens, load the last X messages. Other cases are fine: distill always covers the last 50, so at least 20 are always present. |
| 4 | P2 | `message_manager_test.rs` | Test: a new act should fetch the last X messages from past acts for the prompt. |
| 5 | P2 | `director.rs` | If the narrator spoke the last message, wait for at least 10 messages before it speaks again. |
| 6 | P2 | `director.rs` | Pick among many no-target candidates using CPU embedding. |
| 7 | P3 | `message_manager.rs:send_message` | Check if a second LLM message makes sense (many characters talk per turn). |
| 8 | P3 | `context/mod.rs` | Drop `conn` from params. Pass everything needed via params instead. |
| 9 | P3 | `chat_context.rs:build_closure_prompt` | `current_act_messages_max_threshold` will come from adventure settings later. For now it defaults to 200. |
| 10 | P3 | `state.rs:AppState` | Remove some fields from state. Slint can hold its own state for some of them. |
