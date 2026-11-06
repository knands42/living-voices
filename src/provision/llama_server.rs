use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

use flate2::read::GzDecoder;

use crate::provision::helper::{
    LLAMA_ARCHIVE_FILE, LLAMA_SERVER_URL, human_bytes, provision_llama_archive_path,
    provision_llama_path,
};

/// Print a progress line at most once per this many bytes.
const PROGRESS_EVERY_BYTES: u64 = 16 * 1024 * 1024;

pub enum LlamaStep {
    Fetch,
    Extract,
}

impl LlamaStep {
    fn is_done(&self) -> bool {
        match self {
            LlamaStep::Fetch => provision_llama_archive_path()
                .join(LLAMA_ARCHIVE_FILE)
                .exists(),

            LlamaStep::Extract => provision_llama_path().exists(),
        }
    }
}

pub enum Event {
    Skipped,
    Started,
    Bytes(String),
    Finished,
}

impl Event {
    pub fn describe(&self) {
        match self {
            Event::Skipped => println!("Step is already done"),
            Event::Started => println!("Llama setup is starting"),
            Event::Bytes(b) => println!("Llama downloaded {b}"),
            Event::Finished => println!("Llama setup is finished"),
        }
    }
}

pub async fn run_llama_pipeline<F: FnMut(Event)>(steps: &[LlamaStep], mut on_change: F) -> i32 {
    for step in steps {
        if step.is_done() {
            on_change(Event::Skipped);
            continue;
        }

        on_change(Event::Started);

        match step {
            LlamaStep::Fetch => {
                let code = download(&mut on_change).await;
                if code != 0 {
                    return code;
                }

                on_change(Event::Finished);
            }
            LlamaStep::Extract => {
                extract(&mut on_change);

                on_change(Event::Finished);
            }
        }
    }

    0
}

/// Download the llama.cpp release archive into `provision_llama_archive_path()`.
///
/// Bytes land in `<archive>.part` and are renamed on success, so `is_done`
/// cannot mistake a half-written archive for a finished one.
pub async fn download<F: FnMut(Event)>(on_change: &mut F) -> i32 {
    let url = LLAMA_SERVER_URL.to_owned();

    let archive = provision_llama_archive_path().join(LLAMA_ARCHIVE_FILE);
    let partial_file = archive.with_file_name(format!("{LLAMA_ARCHIVE_FILE}.part"));

    if let Some(parent) = partial_file.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        eprintln!("could not create {}: {err}", parent.display());
        return 1;
    }

    let mut response = match reqwest::get(&url).await {
        Ok(response) => response,
        Err(err) => {
            eprintln!("could not reach {url}: {err}");
            return 1;
        }
    };

    if !response.status().is_success() {
        eprintln!("{url} answered with HTTP {}", response.status());
        return 1;
    }

    let total = response.content_length();
    let mut file = match tokio::fs::File::create(&partial_file).await {
        Ok(file) => file,
        Err(err) => {
            eprintln!("could not create {}: {err}", partial_file.display());
            return 1;
        }
    };

    let mut written = 0;
    let mut reported = 0;

    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(err) => {
                eprintln!("download broke after {written} bytes: {err}");
                return 1;
            }
        };

        if let Err(err) = file.write_all(&chunk).await {
            eprintln!("could not write {}: {err}", partial_file.display());
            return 1;
        }

        written += chunk.len() as u64;

        if written - reported >= PROGRESS_EVERY_BYTES {
            reported = written;
            on_change(Event::Bytes(progress(written, total)));
        }
    }

    if let Err(err) = file.flush().await {
        eprintln!("could not flush {}: {err}", partial_file.display());
        return 1;
    }
    drop(file);

    if let Err(err) = fs::rename(&partial_file, &archive) {
        eprintln!(
            "could not move {} to {}: {err}",
            partial_file.display(),
            archive.display()
        );
        return 1;
    }

    on_change(Event::Bytes(progress(written, Some(written))));
    0
}

fn progress(written: u64, total: Option<u64>) -> String {
    match total {
        Some(total) => format!("{} / {}", human_bytes(written), human_bytes(total)),
        None => human_bytes(written),
    }
}

pub fn extract<F: FnMut(Event)>(_on_change: &mut F) -> i32 {
    let archive = provision_llama_archive_path().join(LLAMA_ARCHIVE_FILE);
    if !archive.is_file() {
        eprintln!(
            "no archive at {}: run the fetch step first",
            archive.display()
        );
        return 1;
    }

    // Unpack beside the archive first, so a failed run never leaves a
    // half-filled install dir for `is_done` to accept.
    let staging = provision_llama_archive_path().join("staging");
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }

    if let Err(err) = fs::create_dir_all(&staging) {
        eprintln!("could not create {}: {err}", staging.display());
        return 1;
    }

    if let Err(err) = unpack(&archive, &staging) {
        eprintln!("{err}");
        let _ = fs::remove_dir_all(&staging);
        return 1;
    }

    let source = unpacked_root(&staging);
    let target = provision_llama_path();

    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }

    if let Some(parent) = target.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        eprintln!("could not create {}: {err}", parent.display());
        let _ = fs::remove_dir_all(&staging);
        return 1;
    }

    if let Err(err) = fs::rename(&source, &target) {
        eprintln!(
            "could not move {} to {}: {err}",
            source.display(),
            target.display()
        );
        let _ = fs::remove_dir_all(&staging);
        return 1;
    }

    // remove leftovers
    let _ = fs::remove_dir_all(&staging);

    0
}

pub fn unpack(archive: &Path, into: &Path) -> Result<(), String> {
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !name.eq(LLAMA_ARCHIVE_FILE) {
        return Err(format!(
            "expected {LLAMA_ARCHIVE_FILE}, got {}",
            archive.display()
        ));
    }

    let file = File::open(archive)
        .map_err(|err| format!("could not open {}: {err}", archive.display()))?;

    let mut tar = tar::Archive::new(GzDecoder::new(file));
    tar.unpack(into)
        .map_err(|err| format!("could not unpack into {}: {err}", into.display()))
}

/// A release archive ships exactly one top-level folder (`llama-b12110/`,
/// `llama.cpp-0.9.0/`). Flattening it keeps the install path stable across
/// versions, so the next step never has to know the tag number.
fn unpacked_root(staging: &Path) -> PathBuf {
    let Ok(entries) = fs::read_dir(staging) else {
        return staging.to_path_buf();
    };

    let mut directories = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir());

    match (directories.next(), directories.next()) {
        (Some(single), None) => single,
        _ => staging.to_path_buf(),
    }
}
