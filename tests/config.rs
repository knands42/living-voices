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
