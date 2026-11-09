use std::path::PathBuf;

use living_voices::config::{
    Config, ImageModelChoice, Settings, Setup, SetupModels, SupportedModel,
};

fn blank_config() -> Config {
    Config {
        settings: Settings::default(),
        setup: Setup::default(),
        config_dir: PathBuf::new(),
        config_file: PathBuf::new(),
        setup_file: PathBuf::new(),
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
fn the_gate_answer_overrides_the_shipped_image_switch() {
    // given the shipped switch on the heavier graph
    let config = blank_config();
    assert!(config.settings.models.image.enabled);
    assert_eq!(
        config.settings.models.image.workflow,
        SupportedModel::FluxQ8
    );

    // when the player declined and the card only fit the lighter one
    let config = config.with_setup(Setup {
        models: SetupModels {
            image: Some(ImageModelChoice {
                enabled: false,
                workflow: SupportedModel::FluxQ4,
            }),
        },
    });

    // then the answer wins over `config.toml` without editing it
    assert!(!config.settings.models.image.enabled);
    assert_eq!(
        config.settings.models.image.workflow,
        SupportedModel::FluxQ4
    );
    assert!(!config.needs_setup());
}

#[test]
fn an_unanswered_machine_still_needs_the_gate() {
    // given a config whose setup file was never written
    // when
    let config = blank_config();

    // then
    assert!(config.needs_setup());
    assert_eq!(config.media_choice(), None);
}

#[test]
fn saving_the_answer_writes_the_marker_where_the_next_run_finds_it() {
    // given a config pointing its setup file at a scratch directory
    let dir = std::env::temp_dir().join(format!("lv_setup_{}", uuid::Uuid::new_v4()));
    let mut config = blank_config();
    config.setup_file = dir.join("setup.toml");

    // when the player accepts on a comfortable card
    config
        .save_setup(ImageModelChoice {
            enabled: true,
            workflow: SupportedModel::FluxQ8,
        })
        .expect("save the answer");

    let written = std::fs::read_to_string(dir.join("setup.toml")).expect("read setup.toml");
    let reloaded: Setup = toml::from_str(&written).expect("parse setup.toml");
    std::fs::remove_dir_all(&dir).expect("clean up");

    // then the file reads like the `config.toml` table it overrides
    assert_eq!(
        written.trim(),
        "[models.image]\nenabled = true\nworkflow = \"fluxq8\""
    );

    // and the running config follows it
    assert_eq!(
        reloaded.media_choice(),
        Some(ImageModelChoice {
            enabled: true,
            workflow: SupportedModel::FluxQ8,
        })
    );
    assert!(config.settings.models.image.enabled);
    assert_eq!(
        config.settings.models.image.workflow,
        SupportedModel::FluxQ8
    );
    assert!(!config.needs_setup());
}
