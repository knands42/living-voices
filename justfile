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
    sudo pacman -S vulkan-headers spirv-headers

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
# Vulkan, not CUDA: one build serves NVIDIA, AMD and Intel, it needs no CUDA toolkit
# at build time, and no libcudart/libcublas at run time for the people we ship to.
# Requires vulkan-headers, spirv-headers and glslc. Add -DGGML_CUDA=ON (plus the
# toolkit and a GCC that nvcc accepts) only if you want CUDA's prefill on NVIDIA.
llama_cmake_flags := env_var_or_default("LLAMA_CMAKE_FLAGS", "-DBUILD_SHARED_LIBS=OFF -DGGML_VULKAN=ON -DGGML_NATIVE=ON -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF")

# What the app itself expects on disk at runtime: {base_dir}/llama, which in a
# debug build is this project root.
llama_runtime := "llama"
llama_runtime_bin := llama_runtime / "llama-server"
model_dir := llama_runtime / "models"
model_roleplay_url := env_var_or_default("MODEL_ROLEPLAY_URL", "https://huggingface.co/bartowski/L3-8B-Stheno-v3.2-GGUF/resolve/main/L3-8B-Stheno-v3.2-Q5_K_M.gguf")
model_roleplay_name := "L3-8B-Stheno-v3.2-Q5_K_M.gguf"
model_memory_url := env_var_or_default("MODEL_MEMORY_URL", "https://huggingface.co/sennaLLMLearner/qwen2.5-7b-memory-distiller/resolve/main/qwen2.5-7b-memory-distiller.Q4_K_M.gguf")
model_memory_name := "qwen2.5-7b-memory-distiller.Q4_K_M.gguf"

# ComfyUI is python: the checkout is the artifact, the venv is the "build", and
# both live in the runtime dir the app expects, {base_dir}/comfyui, so custom
# nodes and models sit where the app reads them.
#
# Release ships the same three things this tree uses, and nothing else:
#   comfyui-<version>.tar.zst   pinned checkout + the interpreter (~120 MB)
#   comfy-requirements.lock     exact versions, generated from a working venv
#   the binary
# The 7 GB of torch/nvidia wheels are installed on the user's machine on first
# run, into comfyui/venv, from that lock. comfy-cli stays for model downloads.
comfy_repo := env_var_or_default("COMFY_REPO", "https://github.com/comfyanonymous/ComfyUI")
comfy_version := env_var_or_default("COMFY_VERSION", "v0.44.1")
comfy_commit := env_var_or_default("COMFY_COMMIT", "49b61c3cd044fb65acd97a960467a00414694146")
comfy_torch_index := env_var_or_default("COMFY_TORCH_INDEX", "https://download.pytorch.org/whl/cu130")
comfy_arch := env_var_or_default("COMFY_ARCH", "x86_64-unknown-linux-gnu")

# CPython from python-build-standalone: unpacks anywhere and keeps working, so
# the engine never borrows the host's python. This is the interpreter the app
# spawns, and the one every venv below is built from.
comfy_python_version := env_var_or_default("COMFY_PYTHON_VERSION", "3.12.16")
comfy_python_minor := env_var_or_default("COMFY_PYTHON_MINOR", "3.12")
comfy_python_release := env_var_or_default("COMFY_PYTHON_RELEASE", "20261026")
comfy_python_asset := "cpython-" + comfy_python_version + "+" + comfy_python_release + "-" + comfy_arch + "-install_only_stripped.tar.gz"
comfy_python_url := env_var_or_default("COMFY_PYTHON_URL", "https://github.com/astral-sh/python-build-standalone/releases/download/" + comfy_python_release + "/" + comfy_python_asset)
comfy_python_sha256 := env_var_or_default("COMFY_PYTHON_SHA256", "ce4f6d0a6cfdb3bed6e44c2f9affd85943c76a42e25ad3a259002f31d6fb8a20")
comfy_python_tarball := provision_archive / "comfy_archive" / comfy_python_asset

comfy_runtime := "comfyui"
comfy_main := comfy_runtime / "main.py"
comfy_ws := comfy_runtime
comfy_python := comfy_runtime / "python" / "bin" / "python3"
comfy_venv := comfy_runtime / "venv"
comfy_bin := comfy_venv / "bin" / "comfy"
# absolute, because comfy-start runs from inside the engine dir
comfy_venv_python := justfile_directory() / comfy_venv / "bin" / "python"
# hashed lock: every pin carries the sha256 of the wheel it may install, so a
# mirrored or hijacked index cannot swap a package under the same version
comfy_lock := "comfy-requirements.lock"
comfy_lock_pins := provision_dir / "comfy-pins.txt"
comfy_runtime_lock := comfy_runtime / comfy_lock
comfy_pip_version := env_var_or_default("COMFY_PIP_VERSION", "26.3")
comfy_bundle := provision_dir / ("comfyui-" + comfy_version + ".tar.zst")
comfy_log_file := "comfy_ui.log"
comfy_log := comfy_runtime / comfy_log_file
comfy_host := "127.0.0.1"
comfy_port := "8188"
comfy_url := "http://" + comfy_host + ":" + comfy_port

# What the tree already has and what is still missing
provision-status:
    @echo "runtime dir: $(pwd)"
    @test -x {{ llama_runtime_bin }} && echo "  llama-server: ok" || echo "  llama-server: missing -> just provision-llama-install"
    @test -f {{ model_dir }}/{{ model_roleplay_name }} && echo "  roleplay model: ok" || echo "  roleplay model: missing -> just provision-llama-model"
    @test -f {{ model_dir }}/{{ model_memory_name }} && echo "  memory model: ok" || echo "  memory model: missing -> just provision-llama-model"
    @test -f {{ comfy_main }} && echo "  ComfyUI:      ok" || echo "  ComfyUI:      missing -> just provision-comfy"
    @test -x {{ comfy_python }} && echo "  comfy python: ok ($({{ comfy_python }} --version))" || echo "  comfy python: missing -> just provision-comfy"
    @test -x {{ comfy_venv }}/bin/python && echo "  comfy venv:   ok" || echo "  comfy venv:   missing -> just provision-comfy"
    @if [ -x {{ comfy_venv }}/bin/python ] && ! grep -q '{{ comfy_python }}' {{ comfy_venv }}/pyvenv.cfg 2>/dev/null; then echo "  comfy venv:   built from the host python -> just provision-clean && just provision-comfy"; fi
    @test -f {{ comfy_runtime_lock }} && echo "  comfy lock:   ok ($(grep -c 'hash=' {{ comfy_runtime_lock }}) hashes)" || echo "  comfy lock:   missing -> just provision-comfy"
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
    test -f {{ model_dir }}/{{ model_roleplay_name }} || curl -fL --continue-at - -o {{ model_dir }}/{{ model_roleplay_name }} {{ model_roleplay_url }}
    test -f {{ model_dir }}/{{ model_memory_name }} || curl -fL --continue-at - -o {{ model_dir }}/{{ model_memory_name }} {{ model_memory_url }}
    @echo "models: {{ model_roleplay_name }}, {{ model_memory_name }}"

provision-llama: provision-llama-install provision-llama-model

# -------------------- comfyui (image/video/audio engine) -------------------- #
# Base engine only; models live in comfy-image.just / comfy-video.just /
# comfy-audio.just. Same split as llama.cpp: source, interpreter, deps, install.

# a tree extracted from the release bundle has no .git, only .comfy-source
provision-comfy-fetch:
    test -f {{ comfy_main }} || git clone --filter=blob:none --no-checkout {{ comfy_repo }} {{ comfy_runtime }}
    @if test -d {{ comfy_runtime }}/.git; then \
        git -C {{ comfy_runtime }} fetch --depth 1 origin {{ comfy_commit }} && \
        git -C {{ comfy_runtime }} checkout --detach --quiet FETCH_HEAD && \
        test "$(git -C {{ comfy_runtime }} rev-parse HEAD)" = "{{ comfy_commit }}" || \
        (echo "checkout is not at {{ comfy_commit }}" && exit 1); \
    else \
        grep -q "{{ comfy_commit }}" {{ comfy_runtime }}/.comfy-source || \
        (echo "{{ comfy_runtime }} is not {{ comfy_commit }} -> just provision-clean" && exit 1); \
    fi
    @echo "source: {{ comfy_runtime }} @ {{ comfy_version }} ({{ comfy_commit }})"

provision-comfy-python:
    mkdir -p {{ provision_archive }}/comfy_archive
    test -f {{ comfy_python_tarball }} || curl -fL {{ comfy_python_url }} -o {{ comfy_python_tarball }}
    echo "{{ comfy_python_sha256 }}  {{ comfy_python_tarball }}" | sha256sum --check
    test -x {{ comfy_python }} || tar -xzf {{ comfy_python_tarball }} -C {{ comfy_runtime }}
    test -x {{ comfy_python }}
    @echo "interpreter: {{ comfy_python }} ($({{ comfy_python }} --version))"

provision-comfy-venv: provision-comfy-fetch provision-comfy-python
    test -f {{ comfy_lock }} || (echo "no {{ comfy_lock }} -> just provision-comfy-lock" && exit 1)
    test -x {{ comfy_venv }}/bin/python || {{ comfy_python }} -m venv {{ comfy_venv }}
    cp {{ comfy_lock }} {{ comfy_runtime_lock }}
    test "$({{ comfy_venv }}/bin/pip --version | cut -d' ' -f2)" = "{{ comfy_pip_version }}" || {{ comfy_venv }}/bin/pip install --upgrade "pip=={{ comfy_pip_version }}"
    {{ comfy_venv }}/bin/pip install --require-hashes --requirement {{ comfy_runtime_lock }} --extra-index-url {{ comfy_torch_index }}
    @echo "env: {{ comfy_venv }} ($({{ comfy_venv }}/bin/python --version), torch $({{ comfy_venv }}/bin/python -c 'import torch; print(torch.__version__)'))"

provision-comfy-install: provision-comfy-venv
    test -f {{ comfy_main }}
    test -d {{ comfy_runtime }}/custom_nodes
    test -f {{ comfy_runtime_lock }}
    @echo "engine: {{ comfy_runtime }} ({{ comfy_version }}, locked deps, interpreter inside)"

provision-comfy: provision-comfy-install

provision-comfy-lock:
    mkdir -p {{ provision_dir }}
    {{ comfy_venv }}/bin/pip freeze --exclude-editable > {{ comfy_lock_pins }}
    {{ comfy_venv }}/bin/uv pip compile \
        --generate-hashes \
        --no-header \
        --no-build \
        --python-version {{ comfy_python_minor }} \
        --index-strategy unsafe-best-match \
        --extra-index-url {{ comfy_torch_index }} \
        {{ comfy_lock_pins }} \
        --output-file {{ comfy_lock }}
    rm -f {{ comfy_lock_pins }}
    @echo "lock: {{ comfy_lock }} ($(grep -c '^[a-zA-Z0-9]' {{ comfy_lock }}) pins, $(grep -c 'hash=' {{ comfy_lock }}) hashes)"

# release artifact: source + interpreter, no venv, no models
provision-comfy-bundle: provision-comfy-install
    echo "{{ comfy_repo }} {{ comfy_commit }} ({{ comfy_version }})" > {{ comfy_runtime }}/.comfy-source
    tar --create --zstd --file {{ comfy_bundle }} \
        --exclude=./venv --exclude=./models --exclude=./output --exclude=./input --exclude=./user \
        --exclude=./.git --exclude=__pycache__ --exclude=./comfy_ui.log \
        --directory {{ comfy_runtime }} .
    sha256sum {{ comfy_bundle }} > {{ comfy_bundle }}.sha256
    @echo "bundle: {{ comfy_bundle }} ($(du -h {{ comfy_bundle }} | cut -f1)), lock: {{ comfy_lock }}"

comfy-wait:
    until curl -sf {{ comfy_url }}/system_stats > /dev/null 2>&1; do sleep 1; done
    @echo "ComfyUI is up at {{ comfy_url }}"

comfy-down:
    -pkill -f "main.py --listen {{ comfy_host }} --port {{ comfy_port }}"
    @echo "ComfyUI stopped"

comfy-start:
    test -f {{ comfy_main }} && test -x {{ comfy_venv }}/bin/python || (echo "engine not provisioned -> just provision-comfy" && exit 1)
    cd {{ comfy_runtime }} && setsid {{ comfy_venv_python }} main.py --listen {{ comfy_host }} --port {{ comfy_port }} --disable-auto-launch {{ env_var_or_default("COMFY_EXTRA", "") }} > {{ comfy_log_file }} 2>&1 < /dev/null &
    @echo "ComfyUI starting on {{ comfy_url }} (log: {{ comfy_log }})"

provision: provision-llama provision-comfy
    @echo "provisioning complete under {{ provision_dir }} and {{ comfy_runtime }}"

chat-test:
    time curl -s http://localhost:11435/v1/chat/completions \
        -H 'Content-Type: application/json' \
        --data-binary @./tests/ai/samples/text/chat-example.json | jq -r .choices[0].message.content

comfy-test workflow="tests/ai/samples/images/image-example-flux-q4.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"
