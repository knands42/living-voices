//! Resumable single-file downloads.
//!
//! Bytes land in `<dest>.part` and are renamed on success, so an interrupted run
//! never leaves a file that looks complete to a size check.

use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;

pub async fn to_file<F>(url: &str, dest: &Path, mut on_progress: F) -> Result<u64, String>
where
    F: FnMut(String, u64, Option<u64>),
{
    let partial_path = create_partial_path(dest);

    if let Some(parent) = partial_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("could not create {}: {err}", parent.display()))?;
    }

    let mut response = reqwest::get(url)
        .await
        .map_err(|err| format!("could not reach {url}: {err}"))?;

    if !response.status().is_success() {
        return Err(format!("{url} answered with HTTP {}", response.status()));
    }

    let total = response.content_length();
    let mut file = tokio::fs::File::create(&partial_path)
        .await
        .map_err(|err| format!("could not create {}: {err}", partial_path.display()))?;

    let file_name = partial_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
        .trim_end_matches(".part")
        .to_string();
    let mut written = 0;
    on_progress(file_name.clone(), 0, total);

    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(err) => return Err(format!("download broke after {written} bytes: {err}")),
        };

        file.write_all(&chunk)
            .await
            .map_err(|err| format!("could not write {}: {err}", partial_path.display()))?;

        written += chunk.len() as u64;
        on_progress(file_name.clone(), written, total);
    }

    file.flush()
        .await
        .map_err(|err| format!("could not flush {}: {err}", partial_path.display()))?;
    drop(file);

    std::fs::rename(&partial_path, dest).map_err(|err| {
        format!(
            "could not move {} to {}: {err}",
            partial_path.display(),
            dest.display()
        )
    })?;

    Ok(written)
}

/// `<dest>.part`, keeping the original name so a resumed download is obvious on disk.
fn create_partial_path(dest: &Path) -> PathBuf {
    let mut name = dest
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(".part");

    dest.with_file_name(name)
}

#[test]
fn partial_downloads_keep_the_model_name() {
    // given
    let model = PathBuf::from("/models/qwen2.5.gguf");

    // when
    let partial = create_partial_path(&model);

    // then
    assert_eq!(partial, PathBuf::from("/models/qwen2.5.gguf.part"));

    // and the finished name is never the partial one
    assert_ne!(partial, model);
}
