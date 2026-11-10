use crate::config::base_dir;
use std::path::PathBuf;

pub const TEXT_MODEL_FILE: &str = "L3-8B-Stheno-v3.2-Q5_K_M.gguf";
pub const TEXT_MODEL_URL: &str = "https://huggingface.co/bartowski/L3-8B-Stheno-v3.2-GGUF/resolve/main/L3-8B-Stheno-v3.2-Q5_K_M.gguf";
pub const MODEL_ALIAS: &str = "L3-8B-Stheno-v3.2-Q5_K_M.gguf";

/// The background summarizer
pub const MEMORY_MODEL_FILE: &str = "qwen2.5-7b-memory-distiller.Q4_K_M.gguf";
pub const MEMORY_MODEL_URL: &str = "https://huggingface.co/sennaLLMLearner/qwen2.5-7b-memory-distiller/resolve/main/qwen2.5-7b-memory-distiller.Q4_K_M.gguf";
pub const MEMORY_MODEL_ALIAS: &str = "qwen2.5-7b-memory-distiller.Q4_K_M.gguf";

/// The engine binary, shipped next to `models/`.
pub const LLAMA_SERVER_BIN: &str = "llama-server";

pub fn llama_dir() -> PathBuf {
    base_dir().join("llama")
}

pub fn llama_models_dir() -> PathBuf {
    llama_dir().join("models")
}

pub fn llama_server_binary() -> PathBuf {
    #[cfg(windows)]
    return llama_dir().join(format!("{LLAMA_SERVER_BIN}.exe"));

    #[cfg(not(windows))]
    llama_dir().join(LLAMA_SERVER_BIN)
}
