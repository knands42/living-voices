use std::path::{Path, PathBuf};

use super::fetch;
use super::helper::{MODEL_FILE, MODEL_URL, llama_models_dir};

pub fn model_path() -> PathBuf {
    llama_models_dir().join(MODEL_FILE)
}

pub fn is_present_at(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name == MODEL_FILE || name == MODEL_ALIAS)
        && path.is_file()
}

pub async fn ensure<F>(on_progress: F) -> Result<PathBuf, String>
where
    F: FnMut(u64, Option<u64>),
{
    let path = model_path();

    if is_present_at(&path) {
        return Ok(path);
    }

    let written = fetch::to_file(MODEL_URL, &path, on_progress).await?;

    if !is_present_at(&path) {
        return Err(format!("{} downloaded {written} bytes", path.display()));
    }

    Ok(path)
}

pub use super::helper::MODEL_ALIAS;
