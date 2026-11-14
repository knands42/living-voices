use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use living_voices::config::base_dir;
use living_voices::provision::comfy::layout::{
    COMFY_COMMIT, COMFY_LOCK_FILE, COMFY_REPO, COMFY_VERSION, comfy_bundle, comfy_custom_nodes_dir,
    comfy_dir, comfy_lock, comfy_main, comfy_python, comfy_venv_python,
};
use living_voices::provision::fetch;
use living_voices::provision::human_bytes;
use sha2::{Digest, Sha256};

use crate::cmd;
use crate::progress::Progress;

const TORCH_INDEX: &str = "https://download.pytorch.org/whl/cu130";
const PIP_VERSION: &str = "26.3";
const PYTHON_VERSION: &str = "3.12.16";
const PYTHON_RELEASE: &str = "20261026";
const PYTHON_ARCH: &str = "x86_64-unknown-linux-gnu";
const PYTHON_SHA256: &str = "ce4f6d0a6cfdb3bed6e44c2f9affd85943c76a42e25ad3a259002f31d6fb8a20";

const COMPRESSION_LEVEL: i32 = 3;
const EXCLUDED_DIRS: &[&str] = &[".git", "input", "models", "output", "user", "venv"];
const EXCLUDED_ANY_LEVEL: &[&str] = &["__pycache__"];
const EXCLUDED_FILES: &[&str] = &["comfy_ui.log"];

pub async fn install() -> Result<(), String> {
    fetch_source()?;
    fetch_python().await?;
    create_venv()?;
    assert_installed()
}

pub async fn bundle() -> Result<(), String> {
    install().await?;
    pack()
}

fn fetch_source() -> Result<(), String> {
    let dir = comfy_dir();
    let repo = crate::env_or("COMFY_REPO", COMFY_REPO);
    let commit = crate::env_or("COMFY_COMMIT", COMFY_COMMIT);

    if !comfy_main().is_file() {
        cmd::run(
            Command::new("git")
                .args(["clone", "--filter=blob:none", "--no-checkout"])
                .arg(&repo)
                .arg(&dir),
            "git clone",
        )?;
    }

    if dir.join(".git").is_dir() {
        cmd::run(
            Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["fetch", "--depth", "1", "origin"])
                .arg(&commit),
            "git fetch",
        )?;

        cmd::run(
            Command::new("git").arg("-C").arg(&dir).args([
                "checkout",
                "--detach",
                "--quiet",
                "FETCH_HEAD",
            ]),
            "git checkout",
        )?;

        let head = cmd::output(
            Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["rev-parse", "HEAD"]),
            "git rev-parse",
        )?;

        if head.trim() != commit {
            return Err(format!(
                "{} is checked out at {}, not {commit}",
                dir.display(),
                head.trim()
            ));
        }
    } else {
        let stamp = dir.join(".comfy-source");
        let body = fs::read_to_string(&stamp)
            .map_err(|err| format!("could not read {}: {err}", stamp.display()))?;

        if !body.contains(&commit) {
            return Err(format!(
                "{} is not {commit} -> `just provision-clean && cargo xtask comfy install`",
                dir.display()
            ));
        }
    }

    println!("source: {} @ {COMFY_VERSION} ({commit})", dir.display());
    Ok(())
}

async fn fetch_python() -> Result<(), String> {
    let python = comfy_python();
    if python.is_file() {
        println!("interpreter: {} (cached)", python.display());
        return Ok(());
    }

    let asset = python_asset();
    let tarball = python_tarball(&asset);

    if !tarball.is_file() {
        if let Some(parent) = tarball.parent() {
            create_dir_all(parent)?;
        }

        let url = python_url(&asset);
        println!("downloading {url}");
        let mut progress = Progress::default();
        fetch::to_file(&url, &tarball, |file, written, total| {
            progress.report(&file, written, total)
        })
        .await?;
        progress.finish();
    }

    let expected = crate::env_or("COMFY_PYTHON_SHA256", PYTHON_SHA256);
    let digest = sha256(&tarball)?;
    if digest != expected {
        return Err(format!(
            "{} hashes to {digest}, expected {expected}",
            tarball.display()
        ));
    }

    extract_tar_gz(&tarball, &comfy_dir())?;

    if !python.is_file() {
        return Err(format!(
            "{} did not unpack {}",
            tarball.display(),
            python.display()
        ));
    }

    println!("interpreter: {}", python.display());
    Ok(())
}

fn create_venv() -> Result<(), String> {
    let source_lock = base_dir().join(COMFY_LOCK_FILE);
    if !source_lock.is_file() {
        return Err(format!(
            "no {} -> generate it with `just provision-comfy-lock`",
            source_lock.display()
        ));
    }

    let venv = comfy_dir().join("venv");
    if !comfy_venv_python().is_file() {
        cmd::run(
            Command::new(comfy_python())
                .arg("-m")
                .arg("venv")
                .arg(&venv),
            "python -m venv",
        )?;
    }

    let runtime_lock = comfy_lock();
    fs::copy(&source_lock, &runtime_lock).map_err(|err| {
        format!(
            "could not copy {} into {}: {err}",
            source_lock.display(),
            runtime_lock.display()
        )
    })?;

    let python = comfy_venv_python();
    let wanted = crate::env_or("COMFY_PIP_VERSION", PIP_VERSION);

    if installed_pip(&python)? != wanted {
        cmd::run(
            Command::new(&python)
                .args(["-m", "pip", "install", "--upgrade"])
                .arg(format!("pip=={wanted}")),
            "pip upgrade",
        )?;
    }

    println!("installing the locked deps (a first run downloads several GB)");
    cmd::run(
        Command::new(&python)
            .args(["-m", "pip", "install", "--require-hashes", "--requirement"])
            .arg(&runtime_lock)
            .arg("--extra-index-url")
            .arg(crate::env_or("COMFY_TORCH_INDEX", TORCH_INDEX)),
        "pip install",
    )
}

fn installed_pip(python: &Path) -> Result<String, String> {
    let version = cmd::output(
        Command::new(python).args(["-m", "pip", "--version"]),
        "pip --version",
    )?;

    version
        .split_whitespace()
        .nth(1)
        .map(str::to_string)
        .ok_or_else(|| format!("could not read the pip version from {version:?}"))
}

fn assert_installed() -> Result<(), String> {
    for file in [comfy_main(), comfy_lock()] {
        if !file.is_file() {
            return Err(format!("{} is missing after install", file.display()));
        }
    }

    let nodes = comfy_custom_nodes_dir();
    if !nodes.is_dir() {
        return Err(format!("{} is missing after install", nodes.display()));
    }

    println!(
        "engine: {} ({COMFY_VERSION}, locked deps, interpreter inside)",
        comfy_dir().display()
    );
    Ok(())
}

fn pack() -> Result<(), String> {
    let source = comfy_dir();
    write_stamp(&source)?;

    let bundle = comfy_bundle();
    if let Some(parent) = bundle.parent() {
        create_dir_all(parent)?;
    }

    if let Err(err) = write_archive(&source, &bundle) {
        let _ = fs::remove_file(&bundle);
        return Err(err);
    }

    let label = label(&bundle);
    write_sha256(&bundle, &label)?;

    let size = fs::metadata(&bundle)
        .map_err(|err| format!("could not read {}: {err}", bundle.display()))?
        .len();

    println!(
        "bundle: {label} ({}), lock: {}",
        human_bytes(size),
        comfy_lock().display()
    );
    Ok(())
}

fn python_asset() -> String {
    format!(
        "cpython-{}+{}-{}-install_only_stripped.tar.gz",
        crate::env_or("COMFY_PYTHON_VERSION", PYTHON_VERSION),
        crate::env_or("COMFY_PYTHON_RELEASE", PYTHON_RELEASE),
        crate::env_or("COMFY_ARCH", PYTHON_ARCH),
    )
}

fn python_url(asset: &str) -> String {
    match std::env::var("COMFY_PYTHON_URL") {
        Ok(url) => url,
        Err(_) => format!(
            "https://github.com/astral-sh/python-build-standalone/releases/download/{}/{asset}",
            crate::env_or("COMFY_PYTHON_RELEASE", PYTHON_RELEASE)
        ),
    }
}

fn python_tarball(asset: &str) -> PathBuf {
    base_dir()
        .join("provision")
        .join("archive")
        .join("comfy_archive")
        .join(asset)
}

fn extract_tar_gz(archive: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive)
        .map_err(|err| format!("could not open {}: {err}", archive.display()))?;
    create_dir_all(destination)?;

    tar::Archive::new(flate2::read::GzDecoder::new(file))
        .unpack(destination)
        .map_err(|err| format!("could not unpack {}: {err}", archive.display()))
}

fn create_dir_all(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|err| format!("could not create {}: {err}", dir.display()))
}

fn write_stamp(source: &Path) -> Result<(), String> {
    let stamp = source.join(".comfy-source");
    let body = format!(
        "{} {} ({})\n",
        crate::env_or("COMFY_REPO", COMFY_REPO),
        crate::env_or("COMFY_COMMIT", COMFY_COMMIT),
        COMFY_VERSION
    );

    fs::write(&stamp, body).map_err(|err| format!("could not write {}: {err}", stamp.display()))
}

fn write_archive(source: &Path, bundle: &Path) -> Result<(), String> {
    let file = File::create(bundle)
        .map_err(|err| format!("could not create {}: {err}", bundle.display()))?;
    let encoder = zstd::Encoder::new(file, COMPRESSION_LEVEL)
        .map_err(|err| format!("could not compress {}: {err}", bundle.display()))?;

    let mut builder = tar::Builder::new(encoder);
    builder.follow_symlinks(false);

    append_tree(&mut builder, source, Path::new("."))?;

    builder
        .into_inner()
        .map_err(|err| format!("could not finish {}: {err}", bundle.display()))?
        .finish()
        .map_err(|err| format!("could not finish {}: {err}", bundle.display()))?;

    Ok(())
}

fn append_tree<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    root: &Path,
    relative: &Path,
) -> Result<(), String> {
    let path = root.join(relative);
    let is_dir = fs::symlink_metadata(&path)
        .map_err(|err| format!("could not read {}: {err}", path.display()))?
        .is_dir();

    builder
        .append_path_with_name(&path, relative)
        .map_err(|err| format!("could not pack {}: {err}", path.display()))?;

    if !is_dir {
        return Ok(());
    }

    let entries =
        fs::read_dir(&path).map_err(|err| format!("could not read {}: {err}", path.display()))?;

    for entry in entries {
        let name = entry
            .map_err(|err| format!("could not read {}: {err}", path.display()))?
            .file_name();
        let child = relative.join(&name);

        if excluded(&child) {
            continue;
        }

        append_tree(builder, root, &child)?;
    }

    Ok(())
}

fn excluded(relative: &Path) -> bool {
    let name = relative
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    if EXCLUDED_ANY_LEVEL.contains(&name.as_str()) {
        return true;
    }

    let top_level = relative.components().count() == 2;
    top_level && (EXCLUDED_DIRS.contains(&name.as_str()) || EXCLUDED_FILES.contains(&name.as_str()))
}

fn write_sha256(bundle: &Path, label: &str) -> Result<(), String> {
    let digest = sha256(bundle)?;
    let checksum = PathBuf::from(format!("{}.sha256", bundle.display()));

    fs::write(&checksum, format!("{digest}  {label}\n"))
        .map_err(|err| format!("could not write {}: {err}", checksum.display()))
}

fn sha256(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|err| format!("could not open {}: {err}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| format!("could not read {}: {err}", path.display()))?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn label(bundle: &Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| bundle.strip_prefix(&cwd).ok().map(Path::to_path_buf))
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|| bundle.display().to_string())
}
