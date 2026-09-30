build:
    cargo build

run-demo: build
    ./target/debug/living_voices --demo

create_migration name:
    sqlx migrate add -r {{name}}
