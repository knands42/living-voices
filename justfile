import 'comfy-image.just'
import 'comfy-video.just'
import 'comfy-audio.just'

build:
    cargo build

lint:
    cargo fmt
    cargo clippy --fix

test:
    cargo test

test-ignored:
    cargo test --test mod -- --ignored --test-threads=1

run-debug: build
    rm *.db* | true
    ./target/debug/living_voices

# Live-reload the whole app
watch:
    cargo watch -x run

# Live-reload a single .slint component
ui-preview component="BuilderPage" file="src/ui/sl/builder/builder_page.slint":
    slint-viewer --auto-reload --component {{ component }} {{ file }}

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

# -------------------- provisioning layout -------------------- #
# Everything the app installs lives under ./provision, relative to this project:
# the same layout a release bundle ships, so development and release share one
# setup. These recipes are the single definition of "install"; the release job
# runs these same commands and packs the result next to the binary.
provision_dir := "provision"
provision_archive := provision_dir / "archive"

llama_archive_dir := provision_archive / "llama_archive"
llama_tarball := llama_archive_dir / "llama.tar.gz"
llama_url := env_var_or_default("LLAMA_URL", "https://github.com/ggml-org/llama.cpp/archive/refs/tags/v0.9.0.tar.gz")
llama_src := provision_dir / "llama"
llama_build := llama_src / "build"
llama_server := llama_build / "bin" / "llama-server"
llama_cmake_flags := env_var_or_default("LLAMA_CMAKE_FLAGS", "-DBUILD_SHARED_LIBS=OFF -DGGML_NATIVE=ON -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF")

# What the app itself expects on disk at runtime: {base_dir}/llama, which in a
# debug build is this project root.
llama_runtime := "llama"
llama_runtime_bin := llama_runtime / "llama-server"
model_dir := llama_runtime / "models"
model_file := "qwen2.5-7b-memory-distiller.Q4_K_M.gguf"
model_path := model_dir / model_file
model_url := env_var_or_default("MODEL_URL", "https://huggingface.co/sennaLLMLearner/qwen2.5-7b-memory-distiller/resolve/main/qwen2.5-7b-memory-distiller.Q4_K_M.gguf")
model_size := "4683073824"

comfy_python := env_var_or_default("COMFY_PYTHON", "python3.12")
comfy_ws := provision_dir / "comfy_ui"
comfy_venv := comfy_ws / "venv"
comfy_bin := comfy_venv / "bin" / "comfy"
comfy_host := "127.0.0.1"
comfy_port := "8188"
comfy_url := "http://" + comfy_host + ":" + comfy_port
comfy_log := provision_dir / "comfy_ui.log"

# What the tree already has and what is still missing
provision-status:
    @echo "runtime root: $(pwd)/{{ llama_runtime }}"
    @test -x {{ llama_runtime_bin }} && echo "  llama-server: ok" || echo "  llama-server: missing -> just provision-llama-install"
    @test -f {{ model_path }} && echo "  model:        ok" || echo "  model:        missing -> just provision-model (the app downloads it too)"
    @test -f {{ comfy_ws }}/main.py && echo "  ComfyUI:      ok" || echo "  ComfyUI:      missing -> just comfy-requirements"
    @test -x {{ comfy_venv }}/bin/python && echo "  comfy venv:   ok" || echo "  comfy venv:   missing -> just comfy-requirements"

provision-clean:
    rm -rf {{ provision_dir }} {{ llama_runtime }}
    @echo "removed {{ provision_dir }} and {{ llama_runtime }}"

# -------------------- llama.cpp (text engine) -------------------- #
provision-llama-fetch:
    mkdir -p {{ llama_archive_dir }}
    test -f {{ llama_tarball }} || curl -fL {{ llama_url }} -o {{ llama_tarball }}
    @echo "archive: {{ llama_tarball }}"

provision-llama-extract: provision-llama-fetch
    test -f {{ llama_src }}/CMakeLists.txt || (rm -rf {{ llama_src }} && mkdir -p {{ llama_src }} && tar -xzf {{ llama_tarball }} -C {{ llama_src }} --strip-components=1)
    @echo "source: {{ llama_src }}"

provision-llama-build: provision-llama-extract
    cmake -B {{ llama_build }} -S {{ llama_src }} {{ llama_cmake_flags }}
    cmake --build {{ llama_build }} --config Release -j --target llama-server
    test -x {{ llama_server }}
    @echo "engine: {{ llama_server }}"

provision-llama-install: provision-llama-build
    mkdir -p {{ llama_runtime }}
    cp {{ llama_server }} {{ llama_runtime_bin }}
    @echo "engine: {{ llama_runtime_bin }}"

provision-llama: provision-llama-install

provision-model:
    mkdir -p {{ model_dir }}
    test -f {{ model_path }} || curl -fL --continue-at - -o {{ model_path }} {{ model_url }}
    test "$(stat -c %s {{ model_path }})" = "{{ model_size }}" || (echo "size mismatch: $(stat -c %s {{ model_path }}) != {{ model_size }}" && exit 1)
    @echo "model: {{ model_path }}"

provision: provision-llama provision-model comfy-requirements
    @echo "provisioning complete under {{ provision_dir }}"

# -------------------- ollama (external alternative) -------------------- #
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

# -------------------- comfy (external alternative) -------------------- #
comfy-requirements:
    {{ comfy_python }} -m venv {{ comfy_venv }}
    {{ comfy_venv }}/bin/pip install --upgrade pip comfy-cli
    {{ comfy_bin }} --workspace {{ comfy_ws }} install --nvidia --restore
    {{ comfy_venv }}/bin/pip install -r {{ comfy_ws }}/requirements.txt
    {{ comfy_venv }}/bin/pip install torch torchvision torchaudio comfy_aimdo comfy_kitchen
    @echo "ComfyUI installed at {{ comfy_ws }} ($({{ comfy_venv }}/bin/python --version))"

comfy-wait:
    until curl -sf {{ comfy_url }}/system_stats > /dev/null 2>&1; do sleep 1; done
    @echo "ComfyUI is up at {{ comfy_url }}"

comfy-down:
    -pkill -f "main.py --listen {{ comfy_host }} --port {{ comfy_port }}"
    @echo "ComfyUI stopped"

comfy-start:
    cd {{ comfy_ws }} && setsid {{ comfy_venv }}/bin/python main.py --listen {{ comfy_host }} --port {{ comfy_port }} --disable-auto-launch {{ env_var_or_default("COMFY_EXTRA", "") }} > {{ comfy_log }} 2>&1 < /dev/null &
    @echo "ComfyUI starting on {{ comfy_url }} (log: {{ comfy_log }})"

comfy-up: comfy-down comfy-start comfy-wait

comfy-test workflow="tests/ai/image-example-flux-q4.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"
