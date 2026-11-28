use std::path::PathBuf;

use living_voices::config::{Config, ConfigHandle, Settings, TextModel};

pub fn chat_config(chat: TextModel) -> ConfigHandle {
    text_config(chat, TextModel::default())
}

pub fn memory_config(memory: TextModel) -> ConfigHandle {
    text_config(TextModel::default(), memory)
}

pub fn text_config(chat: TextModel, memory: TextModel) -> ConfigHandle {
    let mut settings = Settings::default();
    settings.models.chat = chat;
    settings.models.memory = memory;

    ConfigHandle::new(Config {
        settings,
        config_dir: PathBuf::new(),
        config_file: PathBuf::new(),
        db_path: PathBuf::new(),
    })
}
