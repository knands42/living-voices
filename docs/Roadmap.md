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
