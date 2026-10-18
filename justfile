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
