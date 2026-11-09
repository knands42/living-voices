use super::layout::{self, comfy_dir, comfy_lock, comfy_python, comfy_venv_python};
use crate::config::SupportedModel;
use crate::provision::comfy::assets;
use crate::provision::comfy::assets::Asset;
use crate::provision::fetch;
use std::path::Path;
use std::process::Command;

pub enum ComfyEvent {
    InstallingDeps,
    InstallingModel {
        file: String,
        written: u64,
        total: Option<u64>,
    },
}

/// First-run setup: build the venv from the locked deps, then fetch the image model.
pub async fn run_pipeline(mut on_change: impl FnMut(ComfyEvent)) -> Result<(), String> {
    install_deps(&mut on_change)?;
    install_model(SupportedModel::FluxQ4, &mut on_change).await
}

/// Create `comfyui/venv` from the bundled interpreter and install the locked,
/// hash-pinned deps. A no-op once the venv python exists.
fn install_deps(on_change: &mut impl FnMut(ComfyEvent)) -> Result<(), String> {
    if comfy_venv_python().is_file() {
        return Ok(());
    }

    let python = comfy_python();
    if !python.is_file() {
        return Err(format!(
            "no comfy interpreter at {}: extract the bundle first",
            python.display()
        ));
    }

    let lock = comfy_lock();
    if !lock.is_file() {
        return Err(format!("no lock at {}", lock.display()));
    }

    on_change(ComfyEvent::InstallingDeps);
    run_python_command(&python, &["-m", "venv", "venv"])?;

    run_python_command(
        &comfy_venv_python(),
        &[
            "-m",
            "pip",
            "install",
            "--require-hashes",
            "--requirement",
            &lock.to_string_lossy(),
        ],
    )
}

/// Install one image model: clone the GGUF loader node if missing, then fetch
/// every asset not already on disk.
async fn install_model(
    model: SupportedModel,
    on_change: &mut impl FnMut(ComfyEvent),
) -> Result<(), String> {
    ensure_gguf_node()?;

    for asset in Asset::assets(model) {
        let dest = asset.dest();
        if dest.is_file() {
            continue;
        }

        fetch::to_file(asset.url, &dest, |file, written, total| {
            on_change(ComfyEvent::InstallingModel {
                file: file.clone(),
                written,
                total,
            })
        })
        .await?;
    }

    Ok(())
}

/// Every GGUF loader needs city96's custom node plus the `gguf` python package.
fn ensure_gguf_node() -> Result<(), String> {
    let node = layout::comfy_custom_nodes_dir().join(assets::GGUF_NODE_DIR);

    if !node.is_dir() {
        std::fs::create_dir_all(layout::comfy_custom_nodes_dir())
            .map_err(|err| format!("could not create custom_nodes: {err}"))?;

        run_python_command(
            Path::new("git"),
            &[
                "clone",
                "--depth",
                "1",
                assets::GGUF_NODE_REPO,
                &node.to_string_lossy(),
            ],
        )?;
    }

    run_python_command(
        &comfy_venv_python(),
        &["-m", "pip", "install", "--upgrade", "gguf"],
    )
}

fn run_python_command(program: &Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(comfy_dir())
        .status()
        .map_err(|err| format!("could not run {}: {err}", program.display()))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{} {} exited with {status}",
            program.display(),
            args.join(" ")
        ))
    }
}
