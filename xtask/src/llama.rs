use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use living_voices::config::base_dir;
use living_voices::provision::fetch;
use living_voices::provision::human_bytes;
use living_voices::provision::llama::layout::llama_server_binary;

const LLAMA_URL: &str = "https://github.com/ggml-org/llama.cpp/archive/refs/tags/v0.9.0.tar.gz";

// Vulkan, not CUDA: one build serves NVIDIA, AMD and Intel, with no CUDA toolkit
// at build time and no libcudart/libcublas at run time for the people we ship to.
// It needs vulkan-headers, spirv-headers and glslc; -DGGML_CUDA=ON is opt-in.
const CMAKE_FLAGS: &[&str] = &[
    "-DBUILD_SHARED_LIBS=OFF",
    "-DGGML_VULKAN=ON",
    "-DGGML_NATIVE=ON",
    "-DLLAMA_BUILD_TESTS=OFF",
    "-DLLAMA_BUILD_EXAMPLES=OFF",
];

const PROGRESS_STEP: u64 = 8 * 1024 * 1024;

pub async fn fetch() -> Result<(), String> {
    let tarball = tarball();
    if tarball.is_file() {
        println!("archive: {} (cached)", tarball.display());
        return Ok(());
    }

    println!("downloading {}", env_or("LLAMA_URL", LLAMA_URL));
    let mut progress = Progress::default();
    fetch::to_file(
        &env_or("LLAMA_URL", LLAMA_URL),
        &tarball,
        |file, written, total| progress.report(&file, written, total),
    )
    .await?;
    progress.finish();

    println!("archive: {}", tarball.display());
    Ok(())
}

pub fn extract() -> Result<(), String> {
    let source = source_dir();
    if source.join("CMakeLists.txt").is_file() {
        println!("source: {} (cached)", source.display());
        return Ok(());
    }

    let tarball = tarball();
    let staging = staging_dir();
    remove_dir_all(&staging)?;
    create_dir_all(&staging)?;

    let file = fs::File::open(&tarball)
        .map_err(|err| format!("could not open {}: {err}", tarball.display()))?;
    tar::Archive::new(flate2::read::GzDecoder::new(file))
        .unpack(&staging)
        .map_err(|err| format!("could not unpack {}: {err}", tarball.display()))?;

    let root = only_child(&staging)?;
    remove_dir_all(&source)?;
    fs::rename(&root, &source).map_err(|err| {
        format!(
            "could not move {} to {}: {err}",
            root.display(),
            source.display()
        )
    })?;
    remove_dir_all(&staging)?;

    println!("source: {}", source.display());
    Ok(())
}

pub fn build() -> Result<(), String> {
    let source = source_dir();
    if !source.join("CMakeLists.txt").is_file() {
        return Err(format!(
            "{} holds no CMakeLists.txt, run `cargo xtask llama extract` first",
            source.display()
        ));
    }

    run(
        Command::new("cmake")
            .arg("-B")
            .arg(build_dir())
            .arg("-S")
            .arg(&source)
            .args(cmake_flags()),
        "cmake configure",
    )?;

    run(
        Command::new("cmake")
            .arg("--build")
            .arg(build_dir())
            .arg("--config")
            .arg("Release")
            .arg("-j")
            .arg(jobs())
            .arg("--target")
            .arg("llama-server"),
        "cmake build",
    )?;

    let server = built_server();
    if !server.is_file() {
        return Err(format!("cmake did not produce {}", server.display()));
    }

    println!("engine: {}", server.display());
    Ok(())
}

pub fn install() -> Result<(), String> {
    let target = llama_server_binary();
    if let Some(parent) = target.parent() {
        create_dir_all(parent)?;
    }

    fs::copy(built_server(), &target)
        .map_err(|err| format!("could not copy into {}: {err}", target.display()))?;

    println!("engine: {}", target.display());
    Ok(())
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn cmake_flags() -> Vec<String> {
    match std::env::var("LLAMA_CMAKE_FLAGS") {
        Ok(flags) => flags.split_whitespace().map(str::to_string).collect(),
        Err(_) => CMAKE_FLAGS.iter().map(|flag| flag.to_string()).collect(),
    }
}

fn jobs() -> String {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .to_string()
}

fn run(command: &mut Command, what: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|err| format!("{what}: could not start cmake: {err}"))?;

    if !status.success() {
        return Err(format!("{what} exited with {status}"));
    }

    Ok(())
}

fn only_child(dir: &Path) -> Result<PathBuf, String> {
    let mut entries = Vec::new();
    for entry in
        fs::read_dir(dir).map_err(|err| format!("could not read {}: {err}", dir.display()))?
    {
        entries.push(
            entry
                .map_err(|err| format!("could not read {}: {err}", dir.display()))?
                .path(),
        );
    }

    match entries.len() {
        1 if entries[0].is_dir() => Ok(entries.remove(0)),
        _ => Err(format!(
            "{} should hold exactly one source directory",
            dir.display()
        )),
    }
}

fn create_dir_all(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|err| format!("could not create {}: {err}", dir.display()))
}

fn remove_dir_all(dir: &Path) -> Result<(), String> {
    match fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("could not remove {}: {err}", dir.display())),
    }
}

fn provision_dir() -> PathBuf {
    base_dir().join("provision")
}

fn tarball() -> PathBuf {
    provision_dir()
        .join("archive")
        .join("llama_archive")
        .join("llama.tar.gz")
}

fn staging_dir() -> PathBuf {
    provision_dir().join("llama.archive")
}

fn source_dir() -> PathBuf {
    provision_dir().join("llama")
}

fn build_dir() -> PathBuf {
    source_dir().join("build")
}

fn built_server() -> PathBuf {
    let name = llama_server_binary()
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();

    build_dir().join("bin").join(name)
}

#[derive(Default)]
struct Progress {
    printed: u64,
}

impl Progress {
    fn report(&mut self, file: &str, written: u64, total: Option<u64>) {
        let complete = total.is_some_and(|total| written >= total);
        if !complete && written.saturating_sub(self.printed) < PROGRESS_STEP {
            return;
        }

        self.printed = written;
        let line = match total {
            Some(total) => format!("{file}: {} / {}", human_bytes(written), human_bytes(total)),
            None => format!("{file}: {}", human_bytes(written)),
        };

        print!("\r{line}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }

    fn finish(&self) {
        println!();
    }
}
