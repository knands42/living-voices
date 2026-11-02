# -------------------- comfy (audio) -------------------- #
# ComfyUI audio models setup.
# Models: ACE-Step music generation (native in ComfyUI)
# F5-TTS for voice cloning + sentiment

# ACE-Step v1 3.5B all-in-one checkpoint (model + encoders bundled).
ace_url := "https://huggingface.co/Comfy-Org/ACE-Step_ComfyUI_repackaged/resolve/main/all_in_one/ace_step_v1_3.5b.safetensors"

# download the ACE-Step checkpoint (skip if present)
audio-model-ace:
    test -f {{ comfy_ws }}/models/checkpoints/$(basename {{ ace_url }}) && echo "ACE-Step already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ ace_url }} --relative-path models/checkpoints

# queue an ACE-Step music workflow
audio-music-test workflow="tests/ai/audio-example-ace.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at $HOME/.living-voices/comfy/ComfyUI/output"

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
    @echo "Output will be generated at $HOME/.living-voices/comfy/ComfyUI/output"

# ---- Higgs Audio v2 (character speech, tag-based emotion) ----
# Via the TTS-Audio-Suite multi-engine node. Heavier than F5 (3B, ~8GB VRAM)
# and a fragile stack: the suite runs it in an isolated worker. Models
# auto-download to ComfyUI/models/TTS/ on first render.
tts_suite := comfy_ws / "custom_nodes" / "TTS-Audio-Suite"

# install the TTS-Audio-Suite node (provides Higgs Audio 2, also F5 and others)
audio-higgs-node:
    test -d {{ tts_suite }} && echo "TTS-Audio-Suite already present" || git clone --depth 1 https://github.com/diodiogod/TTS-Audio-Suite {{ tts_suite }}
    {{ comfy_venv }}/bin/pip install -r {{ tts_suite }}/requirements.txt

# queue a Higgs Audio v2 speech workflow (reference voice in ComfyUI/input/)
audio-higgs-test workflow="tests/ai/audio-example-higgs.json":
    cp tests/ai/audio.wav tests/ai/audio.txt {{ comfy_ws }}/input/
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at $HOME/.living-voices/comfy/ComfyUI/output"
