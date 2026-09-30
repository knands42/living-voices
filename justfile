build:
    cargo build

run-demo: build
    ./target/debug/living_voices --demo
