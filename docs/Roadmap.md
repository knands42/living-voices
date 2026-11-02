# Roadmap

## Media generation (via ComfyUI)

Priority: video first, audio later.

| model                     | use                             | ComfyUI support               | ease   | notes                                                               |
|---------------------------|---------------------------------|-------------------------------|--------|---------------------------------------------------------------------|
| Wan 2.1                   | video (3s loop)                 | native                        | easy   | heaviest / slowest on 8GB; on-command only                          |
| MMAudio                   | video sound                     | custom node                   | medium | takes a video as input                                              |
| F5-TTS                    | character speech (first choice) | custom node (TTS-Audio-Suite) | medium | TTS: text -> spoken voice; lighter, main env, voice cloning         |
| (optional) Higgs Audio v2 | character speech (fallback)     | custom node (TTS-Audio-Suite) | harder | richer emotion, but fragile stack (isolated worker)                 |
| ACE-Step                  | background music (optional)     | native                        | easy   | music gen: empty lyrics = instrumental; lyrics = sung, not dialogue |

## Features

| feature                                  | notes                                               |
|------------------------------------------|-----------------------------------------------------|
| Gallery / Memory                         | showcase all media generated per act, like an album |
| Narrator settings                        |                                                     |
| Benchmark default vs vllm setup          | script maybe                                        |
| CPU embedding for character selection    |                                                     |
| Challenges & achievements (quest score)  | score on quest completion                           |
| Final per-journey prompt about the score | end-of-journey summary                              |