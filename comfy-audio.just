# -------------------- comfy (audio) -------------------- #
# ComfyUI audio models setup.
# Models: ACE-Step 1.5 XL (music, native) + F5-TTS (character speech, voice clone)

# ACE-Step 1.5 XL (4B DiT, commercial-grade) — split files, offloads to RAM.
ace15_base := "https://huggingface.co/Comfy-Org/ace_step_1.5_ComfyUI_files/resolve/main/split_files/diffusion_models/acestep_v1.5_xl_base_bf16.safetensors"
ace15_clip := "https://huggingface.co/Comfy-Org/ace_step_1.5_ComfyUI_files/resolve/main/split_files/text_encoders/qwen_1.7b_ace15.safetensors"
ace15_vae := "https://huggingface.co/Comfy-Org/ace_step_1.5_ComfyUI_files/resolve/main/split_files/vae/ace_1.5_vae.safetensors"

# download ACE-Step 1.5 XL: 4B diffusion model + 1.7B planner encoder + VAE
audio-model-ace15xl:
    test -f {{ comfy_ws }}/models/diffusion_models/$(basename {{ ace15_base }}) && echo "ACE 1.5 XL base present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ ace15_base }} --relative-path models/diffusion_models
    test -f {{ comfy_ws }}/models/text_encoders/$(basename {{ ace15_clip }}) && echo "ACE 1.5 encoder present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ ace15_clip }} --relative-path models/text_encoders
    test -f {{ comfy_ws }}/models/vae/$(basename {{ ace15_vae }}) && echo "ACE 1.5 VAE present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ ace15_vae }} --relative-path models/vae

audio-music-test workflow="tests/ai/audio-example-ace.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"

audio-song-test workflow="tests/ai/audio-example-ace15xl-song.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"

# ---- F5-TTS (character speech) ----
# Install the ComfyUI-F5-TTS custom.
# Put a <name>.wav plus a matching <name>.txt (the words spoken) in ComfyUI/input/.
f5_node := comfy_ws / "custom_nodes" / "ComfyUI-F5-TTS"

# install the F5-TTS node (clone + submodule + requirements)
audio-f5-node:
    test -d {{ f5_node }} && echo "F5-TTS node already present" || git clone --depth 1 https://github.com/niknah/ComfyUI-F5-TTS {{ f5_node }}
    cd {{ f5_node }} && git submodule update --init --recursive
    {{ comfy_venv }}/bin/pip install -r {{ f5_node }}/requirements.txt

# queue an F5-TTS speech workflow
audio-tts-test workflow="tests/ai/audio-example-f5tts.json":
    cp tests/ai/audio.wav tests/ai/audio.txt {{ comfy_ws }}/input/
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"

# ---- IndexTTS-2 (character speech, emotion control) ----
# One voice clip -> 8 emotion dials + emotion-from-text.
index_node := comfy_ws / "custom_nodes" / "ComfyUI-IndexTTS2"

# install the IndexTTS-2 node (clone + requirements; pynini builds on 3.12)
audio-index-node:
    test -d {{ index_node }} && echo "IndexTTS-2 node already present" || git clone --depth 1 https://github.com/snicolast/ComfyUI-IndexTTS2 {{ index_node }}
    {{ comfy_venv }}/bin/pip install -r {{ index_node }}/requirements.txt

# download the IndexTTS-2 release (~8GB) incl. QwenEmotion into checkpoints/
# (needed before first render; the emotion-from-text node needs QwenEmotion)
audio-index-model:
    test -d {{ index_node }}/checkpoints/qwen0.6bemo4-merge && echo "IndexTTS-2 models already present" || env GIT_LFS_SKIP_SMUDGE=0 {{ comfy_venv }}/bin/hf download IndexTeam/IndexTTS-2 --local-dir {{ index_node }}/checkpoints

# queue an IndexTTS-2 speech workflow (reference voice in ComfyUI/input/)
audio-index-test workflow="tests/ai/audio-example-index.json":
    cp tests/ai/audio.wav tests/ai/audio.txt {{ comfy_ws }}/input/
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"


audio-model-all: audio-index-model audio-model-ace15xl audio-index-node