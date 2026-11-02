# -------------------- comfy (audio) -------------------- #
# ComfyUI audio models setup.
# Models: ACE-Step music generation (native in ComfyUI).

# ACE-Step v1 3.5B all-in-one checkpoint (model + encoders bundled).
ace_url := "https://huggingface.co/Comfy-Org/ACE-Step_ComfyUI_repackaged/resolve/main/all_in_one/ace_step_v1_3.5b.safetensors"

# download the ACE-Step checkpoint (skip if present)
audio-model-ace:
    test -f {{ comfy_ws }}/models/checkpoints/$(basename {{ ace_url }}) && echo "ACE-Step already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ ace_url }} --relative-path models/checkpoints

# queue an ACE-Step music workflow
audio-test workflow="tests/ai/audio-example-ace.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at $HOME/.living-voices/comfy/ComfyUI/output"
