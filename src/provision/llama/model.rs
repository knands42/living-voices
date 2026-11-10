use std::path::{Path, PathBuf};

use super::layout::{
    MEMORY_MODEL_FILE, MEMORY_MODEL_URL, TEXT_MODEL_FILE, TEXT_MODEL_URL, llama_models_dir,
};
use crate::provision::fetch;

fn model_path(file: &str) -> PathBuf {
    llama_models_dir().join(file)
}

fn is_present_at(path: &Path, file: &str) -> bool {
    path.file_name().is_some_and(|name| name == file) && path.is_file()
}

pub async fn ensure<F>(file: &str, url: &str, on_progress: F) -> Result<PathBuf, String>
where
    F: FnMut(String, u64, Option<u64>),
{
    let path = model_path(file);

    if is_present_at(&path, file) {
        return Ok(path);
    }

    let written = fetch::to_file(url, &path, on_progress).await?;

    if !is_present_at(&path, file) {
        return Err(format!("{} downloaded {written} bytes", path.display()));
    }

    Ok(path)
}

pub async fn ensure_chat<F>(on_progress: F) -> Result<PathBuf, String>
where
    F: FnMut(String, u64, Option<u64>),
{
    ensure(TEXT_MODEL_FILE, TEXT_MODEL_URL, on_progress).await
}

pub async fn ensure_memory<F>(on_progress: F) -> Result<PathBuf, String>
where
    F: FnMut(String, u64, Option<u64>),
{
    ensure(MEMORY_MODEL_FILE, MEMORY_MODEL_URL, on_progress).await
}
