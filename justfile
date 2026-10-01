build:
    cargo build

lint:
    cargo fmt

test:
    cargo test

run-demo: build
    ./target/debug/living_voices --demo

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
