use std::path::PathBuf;

use living_voices::config::{Config, Settings};

fn blank_config() -> Config {
    Config {
        settings: Settings::default(),
        config_dir: PathBuf::new(),
        config_file: PathBuf::new(),
        db_path: PathBuf::new(),
    }
}

#[test]
fn text_endpoints_route_chat_and_memory_separately() {
    // given a config that still holds the fallback endpoints
    let config = blank_config();
    assert_eq!(
        config.settings.models.chat.base_url,
        "http://localhost:11434/v1"
    );
    assert_eq!(
        config.settings.models.memory.base_url,
        "http://localhost:11434/v1"
    );

    // when pointed at two engines, trailing slashes included
    let config = config.with_text_endpoints("http://127.0.0.1:11435/", "http://127.0.0.1:11436/");

    // then chat and memory keep their own OpenAI surface
    assert_eq!(
        config.settings.models.chat.base_url,
        "http://127.0.0.1:11435/v1"
    );
    assert_eq!(
        config.settings.models.memory.base_url,
        "http://127.0.0.1:11436/v1"
    );
    assert_eq!(
        config.settings.models.image.base_url,
        "http://localhost:8188"
    );
}

#[test]
fn text_endpoints_keep_every_other_text_setting() {
    // given a config with a tuned memory model
    let mut config = blank_config();
    config.settings.models.memory.model = "distiller".to_string();
    config.settings.models.memory.temperature = 0.1;

    // when
    let config = config.with_text_endpoints("http://127.0.0.1:11435", "http://127.0.0.1:11436");

    // then only the endpoint moved
    assert_eq!(config.settings.models.memory.model, "distiller");
    assert_eq!(config.settings.models.memory.temperature, 0.1);
}

#[test]
fn the_code_defaults_still_ask_the_gate() {
    // given the defaults the binary ships, with no file behind them
    // when
    let config = blank_config();

    // then
    assert!(config.needs_setup());
    assert!(config.settings.models.image.enabled);
}

#[test]
fn the_gate_answer_is_written_into_config_toml() {
    // given a config file in a scratch directory
    let dir = std::env::temp_dir().join(format!("lv_config_{}", uuid::Uuid::new_v4()));
    let mut config = blank_config();
    config.config_file = dir.join("config.toml");
    assert!(config.needs_setup());

    // when the player turns the media engine off
    config.set_custom_config(false).expect("write config.toml");

    let written = std::fs::read_to_string(dir.join("config.toml")).expect("read config.toml");
    let reloaded: Settings = toml::from_str(&written).expect("parse config.toml");
    std::fs::remove_dir_all(&dir).expect("clean up");

    // then the answer is a setting in that one file, not a second config
    assert!(!reloaded.models.image.enabled);
    assert!(reloaded.setup.prompted_gate);

    // and the running config agrees with it
    assert!(!config.settings.models.image.enabled);
    assert!(!config.needs_setup());
}
