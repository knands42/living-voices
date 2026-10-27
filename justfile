build:
    cargo build

lint:
    cargo fmt

test:
    cargo test

test-ignored:
    cargo test -- --ignored

run-debug: build
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


# -------------------- testing -------------------- #
send-message:
    time curl -s http://localhost:11434/v1/chat/completions \
        -H 'Content-Type: application/json' \
        --data-binary @./tests/ai/example.json | jq -r .choices[0].message.content


# -------------------- coverage -------------------- #
cov_ignore := '(src/ui/|src/cli/|app\.rs)'

install-requirements:
    rustup component add llvm-tools-preview
    cargo install cargo-llvm-cov

coverage:
    cargo llvm-cov --summary-only --ignore-filename-regex '{{cov_ignore}}'

coverage-html:
    cargo llvm-cov --html --open --ignore-filename-regex '{{cov_ignore}}'

coverage-all:
    cargo llvm-cov --ignore-filename-regex '{{cov_ignore}}' -- --include-ignored
