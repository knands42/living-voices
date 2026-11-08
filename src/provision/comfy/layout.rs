use crate::config::base_dir;
use std::path::PathBuf;

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
