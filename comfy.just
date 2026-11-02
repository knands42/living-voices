# -------------------- comfy (image) -------------------- #
# ComfyUI setup, model downloads, server lifecycle and workflow tests.
# Imported by the root justfile; run any recipe with `just image-...`.

comfy_home := env_var_or_default("COMFY_HOME", env_var("HOME") / ".living-voices" / "comfy")
comfy_venv := comfy_home / "venv"
comfy_ws := comfy_home / "ComfyUI"
comfy_bin := comfy_venv / "bin" / "comfy"
comfy_host := "127.0.0.1"
comfy_port := "8188"
comfy_url := "http://" + comfy_host + ":" + comfy_port
# Checkpoints (override the *_URL vars to use a CivitAI/HF mirror).
sd15_url := env_var_or_default("SD15_URL", "https://huggingface.co/Comfy-Org/stable-diffusion-v1-5-archive/resolve/main/v1-5-pruned-emaonly-fp16.safetensors")
sdxl_url := env_var_or_default("SDXL_URL", "https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0/resolve/main/sd_xl_base_1.0.safetensors")
sdxl_turbo_url := env_var_or_default("SDXL_TURBO_URL", "https://huggingface.co/stabilityai/sdxl-turbo/resolve/main/sd_xl_turbo_1.0_fp16.safetensors")

# Art-style fine-tunes. CivitAI ones read the token from config-local.toml
dreamshaper_url := "https://civitai.com/api/download/models/354657"
juggernaut_url := "https://civitai.com/api/download/models/1759168"
realvis_url := "https://civitai.com/api/download/models/798204"
animagine_url := "https://huggingface.co/cagliostrolab/animagine-xl-4.0/resolve/main/animagine-xl-4.0-opt.safetensors"

# Flux.1 [dev] GGUF — top quality, slower. Q4 fits VRAM; Q8 offloads to RAM.
# Needs the ComfyUI-GGUF custom node + T5/CLIP encoders + VAE (all token-free).
flux_q4_url := "https://huggingface.co/city96/FLUX.1-dev-gguf/resolve/main/flux1-dev-Q4_K_S.gguf"
flux_q8_url := "https://huggingface.co/city96/FLUX.1-dev-gguf/resolve/main/flux1-dev-Q8_0.gguf"
flux_t5_url := "https://huggingface.co/city96/t5-v1_1-xxl-encoder-gguf/resolve/main/t5-v1_1-xxl-encoder-Q5_K_M.gguf"
flux_clip_url := "https://huggingface.co/comfyanonymous/flux_text_encoders/resolve/main/clip_l.safetensors"
flux_vae_url := "https://huggingface.co/Comfy-Org/Lumina_Image_2.0_Repackaged/resolve/main/split_files/vae/ae.safetensors"

# Chroma (Flux retrain, artistic/unfiltered) — reuses Flux T5 + VAE, new unet.
chroma_url := "https://huggingface.co/silveroxides/Chroma-GGUF/resolve/main/chroma-unlocked-v47-flash-heun/chroma-unlocked-v47-flash-heun-Q4_K_S.gguf"
# Qwen-Image (20B, top-tier + readable text) — its own encoder and VAE.
qwen_url := "https://huggingface.co/city96/Qwen-Image-gguf/resolve/main/qwen-image-Q4_K_S.gguf"
qwen_vae_url := "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/resolve/main/split_files/vae/qwen_image_vae.safetensors"
qwen_clip_url := "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/resolve/main/split_files/text_encoders/qwen_2.5_vl_7b_fp8_scaled.safetensors"
# pull the CivitAI token out of config-local.toml (the single source of truth)
civitai_token := `sed -n 's/^civitai_token[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' config-local.toml`

image-requirements:
    python3 -m venv {{ comfy_venv }}
    {{ comfy_venv }}/bin/pip install --upgrade pip comfy-cli
    {{ comfy_bin }} --workspace {{ comfy_ws }} install --skip-prompt
    {{ comfy_venv }}/bin/pip install -r {{ comfy_ws }}/requirements.txt
    @echo "ComfyUI installed at {{ comfy_ws }}"

# download the default SD 1.5 checkpoint into the workspace (skip if present)
image-model-sd15:
    test -f {{ comfy_ws }}/models/checkpoints/$(basename {{ sd15_url }}) && echo "SD 1.5 already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ sd15_url }} --relative-path models/checkpoints

# download the SDXL base checkpoint (~6.9GB) for later use (skip if present)
image-model-sdxl:
    test -f {{ comfy_ws }}/models/checkpoints/$(basename {{ sdxl_url }}) && echo "SDXL base already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ sdxl_url }} --relative-path models/checkpoints

# download the SDXL Turbo checkpoint for fast 1-4 step drafts (skip if present)
image-model-sdxl-turbo:
    test -f {{ comfy_ws }}/models/checkpoints/$(basename {{ sdxl_turbo_url }}) && echo "SDXL Turbo already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ sdxl_turbo_url }} --relative-path models/checkpoints

# ---- art-style fine-tunes (per-adventure choice) ----

# DreamShaper XL (fantasy / painterly) — CivitAI, needs config civitai_token
image-model-dreamshaper:
    test -f {{ comfy_ws }}/models/checkpoints/dreamshaperXL_lightningDPMSDE.safetensors && echo "DreamShaper already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url "{{ dreamshaper_url }}" --filename dreamshaperXL_lightningDPMSDE.safetensors --relative-path models/checkpoints --set-civitai-api-token "{{ civitai_token }}"

# Juggernaut XL (cinematic realism) — CivitAI, needs config civitai_token
image-model-juggernaut:
    test -f {{ comfy_ws }}/models/checkpoints/juggernautXL_ragnarok.safetensors && echo "Juggernaut already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url "{{ juggernaut_url }}" --filename juggernautXL_ragnarok.safetensors --relative-path models/checkpoints --set-civitai-api-token "{{ civitai_token }}"

# RealVisXL V5 (photorealism) — CivitAI, needs config civitai_token
image-model-realvis:
    test -f {{ comfy_ws }}/models/checkpoints/realvisxlV50.safetensors && echo "RealVisXL already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url "{{ realvis_url }}" --filename realvisxlV50.safetensors --relative-path models/checkpoints --set-civitai-api-token "{{ civitai_token }}"

# Animagine XL 4.0 (anime) — HuggingFace, token-free
image-model-animagine:
    test -f {{ comfy_ws }}/models/checkpoints/$(basename {{ animagine_url }}) && echo "Animagine already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ animagine_url }} --relative-path models/checkpoints

# ---- Flux.1 [dev] GGUF (top quality) ----

# install the ComfyUI-GGUF custom node (needed for the GGUF loaders)
image-flux-node:
    test -d {{ comfy_ws }}/custom_nodes/ComfyUI-GGUF && echo "GGUF node already present" || git clone --depth 1 https://github.com/city96/ComfyUI-GGUF {{ comfy_ws }}/custom_nodes/ComfyUI-GGUF
    {{ comfy_venv }}/bin/pip install --upgrade gguf

# download Flux's shared support files: T5 + CLIP-L encoders and the VAE
image-flux-support:
    test -f {{ comfy_ws }}/models/clip/$(basename {{ flux_t5_url }}) && echo "T5 already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_t5_url }} --relative-path models/clip
    test -f {{ comfy_ws }}/models/clip/$(basename {{ flux_clip_url }}) && echo "CLIP-L already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_clip_url }} --relative-path models/clip
    test -f {{ comfy_ws }}/models/vae/flux_ae.safetensors && echo "VAE already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_vae_url }} --filename flux_ae.safetensors --relative-path models/vae

# Flux Q4 (~6.4GB, fits VRAM, ~80s/image on 8GB) — node + support + unet
image-model-flux-q4: image-flux-node image-flux-support
    test -f {{ comfy_ws }}/models/unet/$(basename {{ flux_q4_url }}) && echo "Flux Q4 already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_q4_url }} --relative-path models/unet

# Flux Q8 (~12GB, offloads to RAM, higher quality) — node + support + unet
image-model-flux-q8: image-flux-node image-flux-support
    test -f {{ comfy_ws }}/models/unet/$(basename {{ flux_q8_url }}) && echo "Flux Q8 already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_q8_url }} --relative-path models/unet

# ---- Chroma & Qwen-Image (top quality, slow) ----

# Chroma (~5.3GB unet) — reuses Flux's GGUF node + T5 + VAE; ~3 min/image
image-model-chroma: image-flux-node image-flux-support
    test -f {{ comfy_ws }}/models/unet/$(basename {{ chroma_url }}) && echo "Chroma already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ chroma_url }} --relative-path models/unet

# Qwen-Image (20B, readable text) — GGUF node + its own encoder and VAE; ~3-4 min
image-model-qwen: image-flux-node
    test -f {{ comfy_ws }}/models/unet/$(basename {{ qwen_url }}) && echo "Qwen unet already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ qwen_url }} --relative-path models/unet
    test -f {{ comfy_ws }}/models/vae/$(basename {{ qwen_vae_url }}) && echo "Qwen VAE already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ qwen_vae_url }} --relative-path models/vae
    test -f {{ comfy_ws }}/models/text_encoders/$(basename {{ qwen_clip_url }}) && echo "Qwen encoder already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ qwen_clip_url }} --relative-path models/text_encoders

# download every checkpoint (base + fine-tunes + Flux + Chroma + Qwen)
image-models-all: image-model-sd15 image-model-sdxl image-model-sdxl-turbo image-model-dreamshaper image-model-juggernaut image-model-realvis image-model-animagine image-model-flux-q4 image-model-flux-q8 image-model-chroma image-model-qwen

image-up:
    cd {{ comfy_ws }} && setsid {{ comfy_venv }}/bin/python main.py --listen {{ comfy_host }} --port {{ comfy_port }} --disable-auto-launch > {{ comfy_home }}/comfyui.log 2>&1 < /dev/null &
    @echo "ComfyUI starting on {{ comfy_url }} (log: {{ comfy_home }}/comfyui.log)"

# block until the API answers
image-wait:
    until curl -sf {{ comfy_url }}/system_stats > /dev/null 2>&1; do sleep 1; done
    @echo "ComfyUI is up at {{ comfy_url }}"

image-down:
    -pkill -f "main.py --listen {{ comfy_host }} --port {{ comfy_port }}"
    @echo "ComfyUI stopped"

image-start: image-up image-wait

image-test workflow="tests/ai/image-example-sdxl.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at $HOME/.living-voices/comfy/ComfyUI/output"

image-models:
    curl -sf {{ comfy_url }}/object_info/CheckpointLoaderSimple | jq '.CheckpointLoaderSimple.input.required.ckpt_name[0]'
