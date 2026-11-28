use std::path::PathBuf;

use living_voices::config::{Config, ConfigHandle, ImageModel, Settings, TextModel};

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
    let handle = ConfigHandle::new(config);
    handle.set_text_endpoints("http://127.0.0.1:11435/", "http://127.0.0.1:11436/");
    let settings = handle.settings();

    // then chat and memory keep their own OpenAI surface
    assert_eq!(settings.models.chat.base_url, "http://127.0.0.1:11435/v1");
    assert_eq!(settings.models.memory.base_url, "http://127.0.0.1:11436/v1");
    assert_eq!(settings.models.image.base_url, "http://localhost:8188");
}

#[test]
fn text_endpoints_keep_every_other_text_setting() {
    // given a config with a tuned memory model
    let mut config = blank_config();
    config.settings.models.memory.model = "distiller".to_string();
    config.settings.models.memory.temperature = 0.1;

    // when
    let handle = ConfigHandle::new(config);
    handle.set_text_endpoints("http://127.0.0.1:11435", "http://127.0.0.1:11436");
    let settings = handle.settings();

    // then only the endpoint moved
    assert_eq!(settings.models.memory.model, "distiller");
    assert_eq!(settings.models.memory.temperature, 0.1);
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

#[cfg(debug_assertions)]
#[test]
fn a_local_config_overrides_only_the_keys_it_sets() {
    // given a base config and a local file that changes one key
    let dir = std::env::temp_dir().join(format!("lv_config_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create dir");
    let base = dir.join("config.toml");
    let local = dir.join("config-local.toml");
    std::fs::write(
        &base,
        "[models.chat]\nmodel = \"base-model\"\ntemperature = 0.5\n",
    )
    .expect("write base");
    std::fs::write(&local, "[models.chat]\ntemperature = 0.9\n").expect("write local");

    // when the local layer is applied on top
    let settings = Settings::load_with_override(&base, &local);
    std::fs::remove_dir_all(&dir).expect("clean up");

    // then the local key wins and every untouched base key survives
    assert_eq!(settings.models.chat.temperature, 0.9);
    assert_eq!(settings.models.chat.model, "base-model");
}

#[cfg(debug_assertions)]
#[test]
fn an_invalid_local_config_leaves_the_base_config_untouched() {
    // given a local file whose value has the wrong type
    let dir = std::env::temp_dir().join(format!("lv_config_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create dir");
    let base = dir.join("config.toml");
    let local = dir.join("config-local.toml");
    std::fs::write(
        &base,
        "[models.chat]\nmodel = \"base-model\"\ntemperature = 0.5\n",
    )
    .expect("write base");
    std::fs::write(&local, "[models.chat]\ntemperature = \"hot\"\n").expect("write local");

    // when the local layer is applied on top
    let settings = Settings::load_with_override(&base, &local);
    std::fs::remove_dir_all(&dir).expect("clean up");

    // then the broken layer is dropped and the base still holds
    assert_eq!(settings.models.chat.temperature, 0.5);
    assert_eq!(settings.models.chat.model, "base-model");
}

#[test]
fn a_published_endpoint_reaches_every_holder() {
    // given a handle that more than one holder cloned while the fallback still stood
    let handle = ConfigHandle::new(blank_config());
    let chat_handler = handle.clone();
    let narrator = handle.clone();
    assert_eq!(
        narrator.chat().base_url,
        "http://localhost:11434/v1",
        "a holder starts on the fallback endpoint"
    );

    // when the spawned engine publishes the ports it actually bound
    handle.set_text_endpoints("http://127.0.0.1:11435/", "http://127.0.0.1:11436/");

    // then every holder reads the new endpoint on its next call, not its next rebuild
    assert_eq!(chat_handler.chat().base_url, "http://127.0.0.1:11435/v1");
    assert_eq!(chat_handler.memory().base_url, "http://127.0.0.1:11436/v1");
    assert_eq!(narrator.chat().base_url, "http://127.0.0.1:11435/v1");
}

#[test]
fn a_spawned_endpoint_is_published_without_touching_the_file() {
    // given a handle backed by a config file, and a holder taken before the spawn
    let dir = std::env::temp_dir().join(format!("lv_config_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create dir");
    let mut config = blank_config();
    config.config_dir = dir.clone();
    config.config_file = dir.join("config.toml");
    std::fs::write(&config.config_file, "").expect("write");
    let handle = ConfigHandle::new(config);
    let holder = handle.clone();

    // when a spawned server publishes the ports it bound
    handle.set_chat_endpoint("http://127.0.0.1:11437/");
    handle.set_memory_endpoint("http://127.0.0.1:11438/");

    // then every holder sees them, and the file keeps the fallback: the port is
    // searched again on the next launch, so persisting it would point at a dead one
    assert_eq!(holder.chat().base_url, "http://127.0.0.1:11437/v1");
    assert_eq!(holder.memory().base_url, "http://127.0.0.1:11438/v1");
    assert_eq!(
        std::fs::read_to_string(&handle.config_file()).expect("read"),
        ""
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn adapters_live_in_their_engine_loras_dir_and_the_dir_is_created() {
    // when each model resolves its adapter directory
    let image = ImageModel::default().loras_dir();
    let text = TextModel::default().loras_dir();

    // then each lands beside its engine's models, and exists for the next copy
    assert!(image.ends_with("comfyui/models/loras"), "{image:?}");
    assert!(image.is_dir(), "{image:?}");
    assert!(text.ends_with("llama/loras"), "{text:?}");
    assert!(text.is_dir(), "{text:?}");
}
