import 'comfy-image.just'
import 'comfy-video.just'
import 'comfy-audio.just'

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

# -------------------- comfy (image/video/audio) -------------------- #
comfy-wait:
    until curl -sf {{ comfy_url }}/system_stats > /dev/null 2>&1; do sleep 1; done
    @echo "ComfyUI is up at {{ comfy_url }}"

comfy-down:
    -pkill -f "main.py --listen {{ comfy_host }} --port {{ comfy_port }}"
    @echo "ComfyUI stopped"

comfy-start:
    cd {{ comfy_ws }} && setsid {{ comfy_venv }}/bin/python main.py --listen {{ comfy_host }} --port {{ comfy_port }} --disable-auto-launch > {{ comfy_home }}/comfyui.log 2>&1 < /dev/null &
    @echo "ComfyUI starting on {{ comfy_url }} (log: {{ comfy_home }}/comfyui.log)"

comfy-up: comfy-down comfy-start comfy-wait

comfy-test workflow="tests/ai/image-example-flux-q4.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at $HOME/.living-voices/comfy/ComfyUI/output"
