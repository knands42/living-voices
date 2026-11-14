use std::process::ExitCode;

mod cmd;
mod comfy;
mod llama;
mod progress;

const USAGE: &str = "\
usage: cargo xtask <task> [step]

tasks:
  llama install    fetch, extract, build and install llama-server (default)
  llama fetch      download the llama.cpp source tarball
  llama extract    unpack it into provision/llama
  llama build      configure and build llama-server with cmake

  comfy install    fetch ComfyUI at the pinned commit, unpack python, build the venv
  comfy bundle     install first, then pack comfyui/ into provision/comfyui-<version>.tar.zst (default)
";

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let task = args.next().unwrap_or_default();
    let step = args.next();

    match task.as_str() {
        "llama" => llama_step(step.as_deref().unwrap_or("install")).await,
        "comfy" => comfy_step(step.as_deref().unwrap_or("bundle")).await,
        "help" | "-h" | "--help" | "" => {
            print!("{USAGE}");
            Ok(())
        }
        _ => Err(format!("unknown task `{task}`\n\n{USAGE}")),
    }
}

async fn llama_step(step: &str) -> Result<(), String> {
    match step {
        "fetch" => llama::fetch().await,
        "extract" => llama::extract(),
        "build" => llama::build(),
        "install" => {
            llama::fetch().await?;
            llama::extract()?;
            llama::build()?;
            llama::install()
        }
        _ => Err(format!("unknown llama step `{step}`\n\n{USAGE}")),
    }
}

async fn comfy_step(step: &str) -> Result<(), String> {
    match step {
        "install" => comfy::install().await,
        "bundle" => comfy::bundle().await,
        _ => Err(format!("unknown comfy step `{step}`\n\n{USAGE}")),
    }
}

pub(crate) fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}
