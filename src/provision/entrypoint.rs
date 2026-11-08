use super::extract;
use super::helper::{comfy_bundle, comfy_dir, comfy_main, human_bytes};
use super::llama_server::LlamaServer;
use super::model;
use crate::provision::comfy::comfy_server::ComfyServer;
use crate::provision::comfy::setup;

const PROGRESS_EVERY_BYTES: u64 = 64 * 1024 * 1024;

/// Download the model if needed, start `llama-server` on it, wait until it answers.
pub async fn start_text_engine() -> Result<LlamaServer, String> {
    let mut last_logged = 0;

    let model_path = model::ensure(|written, total| {
        if written.saturating_sub(last_logged) < PROGRESS_EVERY_BYTES {
            return;
        }

        last_logged = written;
        match total {
            Some(total) => log::info!("model: {} / {}", human_bytes(written), human_bytes(total)),
            None => log::info!("model: {}", human_bytes(written)),
        }
    })
    .await?;

    log::info!("model ready at {}", model_path.display());

    let server = LlamaServer::spawn(&model_path, model::MODEL_ALIAS)?;
    server.wait_until_ready().await?;

    log::info!("text engine answering on {}", server.base_url());

    Ok(server)
}

/// First run: extract the shipped bundle, build the venv from the lock, fetch
/// the image model, start ComfyUI, wait until it answers.
pub async fn start_media_engine() -> Result<ComfyServer, String> {
    if !comfy_main().is_file() {
        let bundle = comfy_bundle();
        log::info!("extracting {}", bundle.display());
        tokio::task::spawn_blocking(move || extract::tar_zst(&bundle, &comfy_dir()))
            .await
            .map_err(|err| format!("extract task failed: {err}"))??;
    }

    log::info!("setting up comfy (first run downloads deps and the image model)");
    let mut last_logged = 0;
    setup::run_pipeline(|event| match event {
        setup::Event::InstallingDeps => log::info!("comfy: installing python deps"),
        setup::Event::InstallingModel {
            file,
            written,
            total,
        } => {
            if written == 0 || written.saturating_sub(last_logged) >= PROGRESS_EVERY_BYTES {
                last_logged = written;
                match total {
                    Some(total) => log::info!(
                        "comfy model {file}: {} / {}",
                        human_bytes(written),
                        human_bytes(total)
                    ),
                    None => log::info!("comfy model {file}: {}", human_bytes(written)),
                }
            }
        }
    })
    .await?;

    let server = ComfyServer::spawn()?;
    server.wait_until_ready().await?;

    log::info!("media engine answering on {}", server.base_url());

    Ok(server)
}
