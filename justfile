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

# ComfyUI is python: the checkout itself is the artifact and the venv is the
# "build". It installs straight into the runtime dir the app expects,
# {base_dir}/comfyui, so custom nodes and models sit where the app reads them.
#
# The source is pinned to one upstream commit, not a moving branch, so a clone
# today and a clone in six months run the same ComfyUI. comfy-cli only installs
# "latest stable", which is the drift this avoids; it stays useful for model
# downloads (`comfy_bin ... model download`).
comfy_python := env_var_or_default("COMFY_PYTHON", "python3.12")
comfy_repo := env_var_or_default("COMFY_REPO", "https://github.com/comfyanonymous/ComfyUI")
comfy_version := env_var_or_default("COMFY_VERSION", "v0.44.1")
comfy_commit := env_var_or_default("COMFY_COMMIT", "49b61c3cd044fb65acd97a960467a00414694146")
comfy_torch_index := env_var_or_default("COMFY_TORCH_INDEX", "https://download.pytorch.org/whl/cu130")
comfy_runtime := "comfyui"
comfy_main := comfy_runtime / "main.py"
comfy_ws := comfy_runtime
comfy_venv := comfy_runtime / "venv"
comfy_bin := comfy_venv / "bin" / "comfy"
# absolute, because comfy-start runs from inside the engine dir
comfy_python_bin := justfile_directory() / comfy_venv / "bin" / "python"
comfy_log_file := "comfy_ui.log"
comfy_log := comfy_runtime / comfy_log_file
comfy_host := "127.0.0.1"
comfy_port := "8188"
comfy_url := "http://" + comfy_host + ":" + comfy_port

# What the tree already has and what is still missing
provision-status:
    @echo "runtime dir: $(pwd)"
    @test -x {{ llama_runtime_bin }} && echo "  llama-server: ok" || echo "  llama-server: missing -> just provision-llama-install"
    @test -f {{ model_path }} && echo "  llama model:  ok" || echo "  llama model:  missing -> just provision-llama-model (the app downloads it too)"
    @test -f {{ comfy_main }} && echo "  ComfyUI:      ok" || echo "  ComfyUI:      missing -> just provision-comfy"
    @test -x {{ comfy_venv }}/bin/python && echo "  comfy venv:   ok" || echo "  comfy venv:   missing -> just provision-comfy"
    @test -n "$(ls -A {{ comfy_ws }}/models/unet 2>/dev/null | grep -v '^put_')" && echo "  comfy models: ok" || echo "  comfy models: missing -> just image-models-all (or video-/audio-)"

provision-clean:
    rm -rf {{ provision_dir }} {{ llama_runtime }} {{ comfy_runtime }}
    @echo "removed {{ provision_dir }}, {{ llama_runtime }} and {{ comfy_runtime }}"

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

provision-llama-model:
    mkdir -p {{ model_dir }}
    test -f {{ model_path }} || curl -fL --continue-at - -o {{ model_path }} {{ model_url }}
    test "$(stat -c %s {{ model_path }})" = "{{ model_size }}" || (echo "size mismatch: $(stat -c %s {{ model_path }}) != {{ model_size }}" && exit 1)
    @echo "model: {{ model_path }}"

provision-llama: provision-llama-install provision-llama-model

# -------------------- comfyui (image/video/audio engine) -------------------- #
provision-comfy-fetch:
    test -d {{ comfy_runtime }}/.git || git clone --filter=blob:none --no-checkout {{ comfy_repo }} {{ comfy_runtime }}
    git -C {{ comfy_runtime }} fetch --depth 1 origin {{ comfy_commit }}
    git -C {{ comfy_runtime }} checkout --detach --quiet FETCH_HEAD
    test "$(git -C {{ comfy_runtime }} rev-parse HEAD)" = "{{ comfy_commit }}" || (echo "checkout is not at {{ comfy_commit }}" && exit 1)
    @echo "source: {{ comfy_runtime }} @ {{ comfy_version }} ({{ comfy_commit }})"

provision-comfy-venv: provision-comfy-fetch
    test -x {{ comfy_venv }}/bin/python || {{ comfy_python }} -m venv {{ comfy_venv }}
    {{ comfy_venv }}/bin/pip install --upgrade pip comfy-cli
    {{ comfy_venv }}/bin/pip install -r {{ comfy_runtime }}/requirements.txt
    {{ comfy_venv }}/bin/pip install torch torchvision torchaudio --extra-index-url {{ comfy_torch_index }}
    {{ comfy_venv }}/bin/pip install comfy_aimdo comfy_kitchen
    @echo "engine: {{ comfy_venv }} ($({{ comfy_venv }}/bin/python --version))"

provision-comfy-install: provision-comfy-venv
    test -f {{ comfy_main }}
    test -d {{ comfy_runtime }}/custom_nodes
    @echo "engine: {{ comfy_main }}"

provision-comfy: provision-comfy-install

provision: provision-llama provision-llama-model provision-comfy
    @echo "provisioning complete under {{ provision_dir }} and {{ comfy_runtime }}"

chat-test:
    time curl -s http://localhost:11435/v1/chat/completions \
        -H 'Content-Type: application/json' \
        --data-binary @./tests/ai/chat-example.json | jq -r .choices[0].message.content

comfy-test workflow="tests/ai/image-example-flux-q4.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"
