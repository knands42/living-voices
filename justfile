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
    sqlx migrate add -r {{name}}

# -------------------- database -------------------- #
db := env_var_or_default("DB", "living_voices.db")

db:
    sqlite3 {{db}}

db-tables:
    sqlite3 {{db}} ".tables"

db-reset:
    rm -f {{db}} {{db}}-wal {{db}}-shm
    @echo "removed {{db}}"

# -------------------- coverage -------------------- #
cov_ignore := '(src/ui/|src/cli/|app\.rs)'

dev-requirements:
    rustup component add llvm-tools-preview
    cargo install cargo-llvm-cov

coverage:
    cargo llvm-cov --summary-only --ignore-filename-regex '{{cov_ignore}}'

coverage-html:
    cargo llvm-cov --html --open --ignore-filename-regex '{{cov_ignore}}'

coverage-all:
    cargo llvm-cov --ignore-filename-regex '{{cov_ignore}}' -- --include-ignored

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
# Default SD 1.5 checkpoint (override SD15_URL to use a CivitAI/HF mirror).
sd15_url := env_var_or_default("SD15_URL", "https://huggingface.co/Comfy-Org/stable-diffusion-v1-5-archive/resolve/main/v1-5-pruned-emaonly-fp16.safetensors")

image-requirements:
    python3 -m venv {{comfy_venv}}
    {{comfy_venv}}/bin/pip install --upgrade pip comfy-cli
    {{comfy_bin}} --workspace {{comfy_ws}} install
    @echo "ComfyUI installed at {{comfy_ws}}"

# download the default SD 1.5 checkpoint into the workspace
image-model:
    {{comfy_bin}} --workspace {{comfy_ws}} model download --url {{sd15_url}} --relative-path models/checkpoints
    @echo "SD 1.5 checkpoint downloaded"

# start ComfyUI headless in the background (bound to localhost only).
# Run main.py with the venv python directly: comfy-cli's own --background
# re-spawns `comfy` on PATH, which the venv install is not.
# setsid fully detaches it so it survives this recipe returning.
image-up:
    cd {{comfy_ws}} && setsid {{comfy_venv}}/bin/python main.py --listen {{comfy_host}} --port {{comfy_port}} --disable-auto-launch > {{comfy_home}}/comfyui.log 2>&1 < /dev/null &
    @echo "ComfyUI starting on {{comfy_url}} (log: {{comfy_home}}/comfyui.log)"

# block until the API answers
image-wait:
    until curl -sf {{comfy_url}}/system_stats > /dev/null 2>&1; do sleep 1; done
    @echo "ComfyUI is up at {{comfy_url}}"

# stop the background server (match the listen args; the python argv is just
# `main.py` since image-up cd's into the workspace first)
image-down:
    -pkill -f "main.py --listen {{comfy_host}} --port {{comfy_port}}"
    @echo "ComfyUI stopped"

image-start: image-up image-wait

# queue an API-format workflow (export from ComfyUI via File > Export (API))
image-test workflow="tests/ai/scene.json":
    curl -sf -X POST {{comfy_url}}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{workflow}})"

# list installed checkpoints the API can see
image-models:
    curl -sf {{comfy_url}}/object_info/CheckpointLoaderSimple | jq '.CheckpointLoaderSimple.input.required.ckpt_name[0]'
