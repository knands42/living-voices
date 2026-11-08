use super::comfy::layout::{comfy_bundle, comfy_dir, comfy_main};
use super::llama::layout::{MEMORY_MODEL_ALIAS, MODEL_ALIAS};
use super::util::human_bytes;
use crate::provision::comfy::comfy_server::ComfyServer;
use crate::provision::comfy::setup;
use crate::provision::extract;
use crate::provision::llama::llama_server::{CHAT_PORT, EngineProfile, LlamaServer, MEMORY_PORT};
use crate::provision::llama::model;

const PROGRESS_EVERY_BYTES: u64 = 64 * 1024 * 1024;

/// The two text models the app talks to: a warm chat server and a background
/// distiller, each on its own fixed port so the URLs stay stable.
pub struct TextEngine {
    pub chat: LlamaServer,
    pub memory: LlamaServer,
}

impl TextEngine {
    pub fn chat_url(&self) -> &str {
        self.chat.base_url()
    }

    pub fn memory_url(&self) -> &str {
        self.memory.base_url()
    }
}

/// Download both models if needed, start one `llama-server` per role, wait until
/// both answer.
pub async fn start_text_engine() -> Result<TextEngine, String> {
    let chat_model = model::ensure_chat(progress("chat")).await?;
    let memory_model = model::ensure_memory(progress("memory")).await?;

    log::info!("chat model ready at {}", chat_model.display());
    log::info!("memory model ready at {}", memory_model.display());

    let chat = LlamaServer::spawn(&chat_model, MODEL_ALIAS, CHAT_PORT, &EngineProfile::chat())?;
    let memory = LlamaServer::spawn(
        &memory_model,
        MEMORY_MODEL_ALIAS,
        MEMORY_PORT,
        &EngineProfile::memory(),
    )?;

    chat.wait_until_ready().await?;
    memory.wait_until_ready().await?;

    log::info!("chat engine answering on {}", chat.base_url());
    log::info!("memory engine answering on {}", memory.base_url());

    Ok(TextEngine { chat, memory })
}

/// Download progress keyed by role, so a first run with two models stays readable.
fn progress(role: &'static str) -> impl FnMut(u64, Option<u64>) {
    let mut last_logged = 0;

    move |written, total| {
        if written.saturating_sub(last_logged) < PROGRESS_EVERY_BYTES {
            return;
        }

        last_logged = written;
        match total {
            Some(total) => log::info!(
                "{role} model: {} / {}",
                human_bytes(written),
                human_bytes(total)
            ),
            None => log::info!("{role} model: {}", human_bytes(written)),
        }
    }
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
