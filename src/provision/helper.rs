use crate::config::base_dir;
use std::path::PathBuf;

pub const LLAMA_SERVER_URL: &str =
    "https://github.com/ggml-org/llama.cpp/archive/refs/tags/v0.9.0.tar.gz";

/// Name the archive is saved as inside [`provision_llama_archive_path`].
pub const LLAMA_ARCHIVE_FILE: &str = "llama.tar.gz";

/// The executable a source build produces.
pub const LLAMA_SERVER_BIN: &str = "llama-server";

fn provision_path() -> PathBuf {
    base_dir().join("provision")
}

fn provision_archive_path() -> PathBuf {
    provision_path().join("archive")
}

pub fn provision_llama_archive_path() -> PathBuf {
    provision_archive_path().join("llama_archive")
}

pub fn provision_comfy_ui_archive_path() -> PathBuf {
    provision_archive_path().join("comfy_ui_archive")
}

pub fn provision_llama_path() -> PathBuf {
    provision_path().join("llama")
}

/// Where a `cmake --build` of the source tree puts the binaries.
pub fn llama_server_binary() -> PathBuf {
    provision_llama_path()
        .join("build")
        .join("bin")
        .join(LLAMA_SERVER_BIN)
}

pub fn provision_comfy_ui_path() -> PathBuf {
    provision_path().join("comfy_ui")
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

pub enum GPUVendor {
    Nvidia,
    AMD,
    Intel,
    Unknown,
}

impl PartialEq for GPUVendor {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl GPUVendor {
    #[cfg(target_os = "linux")]
    pub fn detect_vendor() -> GPUVendor {
        GPUVendor::Nvidia
    }

    #[cfg(target_os = "windows")]
    pub fn detect_vendor() -> GPUVendor {
        GPUVendor::Nvidia
    }
}
