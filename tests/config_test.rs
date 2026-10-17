use living_voices::config::Settings;

#[test]
fn shipped_config_parses() {
    let settings: Settings =
        toml::from_str(include_str!("../config.toml")).expect("config.toml must parse");

    assert!(!settings.models.chat.model.is_empty());
    assert!(!settings.models.memory.model.is_empty());
    assert_eq!(settings.models.image.backend, "comfyui");
}

#[test]
fn model_endpoints_default_when_absent() {
    let settings: Settings = toml::from_str("").expect("empty config parses");

    assert_eq!(settings.models.chat.base_url, "http://localhost:11434/v1");
    assert_eq!(settings.models.image.base_url, "http://localhost:8188");
}
