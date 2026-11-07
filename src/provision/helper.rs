use crate::config::base_dir;
use std::path::PathBuf;

pub const MODEL_FILE: &str = "L3-8B-Stheno-v3.2-Q5_K_M.gguf";
pub const MODEL_URL: &str = "https://huggingface.co/bartowski/L3-8B-Stheno-v3.2-GGUF/resolve/main/L3-8B-Stheno-v3.2-Q5_K_M.gguf";
/// What `--alias` reports through `/v1/models`, and what `config.toml` must say.
pub const MODEL_ALIAS: &str = "L3-8B-Stheno-v3.2-Q5_K_M.gguf";

/// The engine binary, shipped next to `models/`.
pub const LLAMA_SERVER_BIN: &str = "llama-server";

/// `{base_dir}/llama`: the runtime engine dir, both in debug (project root) and
/// in a release bundle (next to the executable's data dir).
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

/// Sizes for progress lines: `0 B`, `1023 B`, `1.5 KiB`, `6.0 GiB`.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
