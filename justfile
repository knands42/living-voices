build:
    cargo build

lint:
    cargo fmt

test:
    cargo test

test-ignored:
    cargo test --test mod -- --ignored --test-threads=1

run-debug: build
    rm *.db* | true
    ./target/debug/living_voices

run-release:
    cargo build --release
    ./target/release/living_voices

create-migration name:
    sqlx migrate add -r {{ name }}

# -------------------- database -------------------- #
db := env_var_or_default("DB", "living_voices.db")

db:
    sqlite3 {{ db }}

db-tables:
    sqlite3 {{ db }} ".tables"

db-reset:
    rm -f {{ db }} {{ db }}-wal {{ db }}-shm
    @echo "removed {{ db }}"

# -------------------- coverage -------------------- #
cov_ignore := '(src/ui/|src/cli/|app\.rs)'

dev-requirements:
    rustup component add llvm-tools-preview
    cargo install cargo-llvm-cov
    cp config.toml config-local.toml

coverage:
    cargo llvm-cov --summary-only --ignore-filename-regex '{{ cov_ignore }}'

coverage-html:
    cargo llvm-cov --html --open --ignore-filename-regex '{{ cov_ignore }}'

coverage-all:
    cargo llvm-cov --ignore-filename-regex '{{ cov_ignore }}' -- --include-ignored

# -------------------- llama.cpp (text) -------------------- #
chat-requirements:
    curl -fsSL https://ollama.com/install.sh | sh
    ollama run llama3.1:8b
    ollama run hf.co/sennaLLMLearner/qwen2.5-7b-memory-distiller:Q4_K_M

chat-up:
    ollama serve || true

chat-test:
    time curl -s http://localhost:11434/v1/chat/completions \
        -H 'Content-Type: application/json' \
        --data-binary @./tests/ai/chat-example.json | jq -r .choices[0].message.content

# -------------------- comfy (image) -------------------- #
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

# Art-style fine-tunes. CivitAI ones read the token from config.toml
dreamshaper_url := "https://civitai.com/api/download/models/354657"
juggernaut_url := "https://civitai.com/api/download/models/1759168"
realvis_url := "https://civitai.com/api/download/models/798204"
animagine_url := "https://huggingface.co/cagliostrolab/animagine-xl-4.0/resolve/main/animagine-xl-4.0-opt.safetensors"
# pull the CivitAI token out of config.toml (the single source of truth)
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

# download every checkpoint (base set + all four art-style fine-tunes)
image-models-all: image-model-sd15 image-model-sdxl image-model-sdxl-turbo image-model-dreamshaper image-model-juggernaut image-model-realvis image-model-animagine

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

image-models:
    curl -sf {{ comfy_url }}/object_info/CheckpointLoaderSimple | jq '.CheckpointLoaderSimple.input.required.ckpt_name[0]'
