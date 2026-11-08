use crate::config::base_dir;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

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

pub const COMFY_VERSION: &str = "v0.44.1";
pub const COMFY_LOCK_FILE: &str = "comfy-requirements.lock";
pub const COMFY_HOST: &str = "127.0.0.1";
pub const COMFY_PORT: u16 = 8188;

/// `{base_dir}/comfyui`: the extracted engine tree, holding `main.py`, the
/// bundled `python/` interpreter, the `venv/` built on first run, and the lock.
pub fn comfy_dir() -> PathBuf {
    base_dir().join("comfyui")
}

/// The release artifact shipped next to the binary, extracted into [`comfy_dir`].
pub fn comfy_bundle() -> PathBuf {
    base_dir()
        .join("provision")
        .join(format!("comfyui-{COMFY_VERSION}.tar.zst"))
}

pub fn comfy_main() -> PathBuf {
    comfy_dir().join("main.py")
}

/// The interpreter the bundle ships, every venv is built from this one.
pub fn comfy_python() -> PathBuf {
    comfy_dir().join("python").join("bin").join("python3")
}

/// The venv python the engine actually runs under, absent until first install.
pub fn comfy_venv_python() -> PathBuf {
    comfy_dir().join("venv").join("bin").join("python")
}

/// The hashed lock travels inside the bundle, so installs read it post-extract.
pub fn comfy_lock() -> PathBuf {
    comfy_dir().join(COMFY_LOCK_FILE)
}

/// `comfyui/models`: where every image model asset lands, under the subdir the
/// loader nodes read (`unet`, `clip`, `vae`, `text_encoders`).
pub fn comfy_models_dir() -> PathBuf {
    comfy_dir().join("models")
}

pub fn comfy_custom_nodes_dir() -> PathBuf {
    comfy_dir().join("custom_nodes")
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

pub async fn wait_until_ready(
    base_url: &str,
    ready_pool: Duration,
    ready_timeout: Duration,
    log_file_name: &str,
) -> Result<(), String> {
    let url = format!("{}/system_stats", base_url);
    let deadline = tokio::time::Instant::now() + ready_timeout;

    while tokio::time::Instant::now() < deadline {
        if let Ok(response) = reqwest::get(&url).await
            && response.status().is_success()
        {
            return Ok(());
        }

        tokio::time::sleep(ready_pool).await;
    }

    Err(format!(
        "comfy never answered {url} (waited {ready_timeout:?}, see {log_file_name})"
    ))
}

pub(crate) fn get_available_port(
    preferred_port: u16,
    port_search_limit: u16,
) -> Result<u16, String> {
    let last = preferred_port.saturating_add(port_search_limit);

    (preferred_port..=last)
        .find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .ok_or_else(|| format!("no free port in {preferred_port}..={last}"))
}

pub fn log_file(path: &Path) -> Result<Stdio, String> {
    if let Some(parent) = path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        return Err(format!("could not create {}: {err}", parent.display()));
    }

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(Stdio::from)
        .map_err(|err| format!("could not open {}: {err}", path.display()))
}

#[cfg(test)]
mod test {
    use crate::provision::{PREFERRED_PORT, helper};
    use std::net::TcpListener;

    #[test]
    fn first_free_port_skips_a_taken_port() {
        // given a port somebody else already holds
        let taken =
            TcpListener::bind(format!("127.0.0.1:{}", PREFERRED_PORT)).expect("bind ephemeral");
        let taken_port = taken.local_addr().expect("addr").port();

        // when
        let next = helper::get_available_port(PREFERRED_PORT, 10).expect("a free port");

        // then it moved on instead of answering with the busy one
        assert_ne!(next, taken_port);
        assert!(next > taken_port);
    }
}
