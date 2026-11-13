use super::comfy::layout::{comfy_bundle, comfy_dir, comfy_main};
use super::llama::layout::{
    MEMORY_MODEL_ALIAS, MEMORY_MODEL_FILE, MODEL_ALIAS, TEXT_MODEL_FILE, llama_dir,
    llama_models_dir,
};
use crate::config::Settings;
use crate::provision::comfy::comfy_server::ComfyServer;
use crate::provision::comfy::setup;
use crate::provision::llama::llama_server::{CHAT_PORT, EngineProfile, LlamaServer, MEMORY_PORT};
use crate::provision::llama::model;
use crate::provision::util::human_bytes;
use crate::provision::{ComfyEvent, extract};

const PROGRESS_EVERY_BYTES: u64 = 64 * 1024 * 1024;

/// The two text models the app talks to
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

pub struct MediaEngine {
    pub server: ComfyServer,
}

impl MediaEngine {
    pub fn media_url(&self) -> &str {
        self.server.base_url()
    }
}

/// Start every engine `settings`, never persist the outcome on disk
pub async fn start_provisioning<T, M>(
    settings: &Settings,
    mut text_engine_callback: T,
    mut media_engine_callback: M,
) -> Result<(), String>
where
    T: FnMut(String, u64, Option<u64>),
    M: FnMut(ComfyEvent),
{
    match provision_text_engine(&mut text_engine_callback).await {
        Ok(_) => {
            log::info!("chat & memory engine setup");
        }
        Err(err) => {
            log::warn!("text engine unavailable: {err}");
        }
    };

    if settings.models.image.enabled {
        match provision_media_engine(&mut media_engine_callback).await {
            Ok(_) => {
                log::info!("media engine setup");
            }
            Err(err) => {
                log::warn!("media engine unavailable: {err}");
            }
        }
    }

    Ok(())
}

pub fn default_text_progress_logger() -> impl FnMut(String, u64, Option<u64>) {
    let mut last_logged = 0;

    move |file_name, written, total| {
        if written.saturating_sub(last_logged) < PROGRESS_EVERY_BYTES {
            return;
        }

        last_logged = written;
        match total {
            Some(total) => log::info!(
                "llama model {file_name}: {} / {}",
                human_bytes(written),
                human_bytes(total)
            ),
            None => log::info!("llama model {file_name}: {}", human_bytes(written)),
        }
    }
}

pub fn default_media_progress_logger() -> impl FnMut(ComfyEvent) {
    let mut last_logged = 0;

    move |event| match event {
        ComfyEvent::InstallingDeps => log::info!("comfy: installing python deps"),
        ComfyEvent::InstallingModel {
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
    }
}

/// Download both models if needed
async fn provision_text_engine<F>(on_progress: &mut F) -> Result<(), String>
where
    F: FnMut(String, u64, Option<u64>),
{
    model::ensure_chat(&mut *on_progress).await?;
    model::ensure_memory(&mut *on_progress).await?;
    Ok(())
}

/// Provision comfyui engine
async fn provision_media_engine<M>(media_engine_callback: &mut M) -> Result<(), String>
where
    M: FnMut(ComfyEvent),
{
    if !comfy_main().is_file() {
        let bundle = comfy_bundle();
        log::info!("extracting {}", bundle.display());
        tokio::task::spawn_blocking(move || extract::tar_zst(&bundle, &comfy_dir()))
            .await
            .map_err(|err| format!("extract task failed: {err}"))??;
    }

    log::info!("setting up comfy (first run downloads deps and the image model)");
    setup::run_pipeline(&mut *media_engine_callback).await?;
    Ok(())
}

pub async fn start_text_engine() -> Result<TextEngine, String> {
    let chat_model = llama_models_dir().join(TEXT_MODEL_FILE);
    let memory_model = llama_models_dir().join(MEMORY_MODEL_FILE);

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

pub async fn start_media_engine() -> Result<MediaEngine, String> {
    let server = ComfyServer::spawn()?;
    server.wait_until_ready().await?;

    log::info!("media engine answering on {}", server.base_url());

    Ok(MediaEngine { server })
}

// Holds the server processes: `LlamaServer` and `ComfyServer` kill their child on drop,
// so whoever owns this decides how long the engines live.
pub struct RunningEngines {
    pub text: TextEngine,
    pub media: Option<MediaEngine>,
}

impl RunningEngines {
    // Both servers search for a free port, so the real endpoints only exist after spawn.
    pub fn stamp(&self, config: crate::config::Config) -> crate::config::Config {
        let config = config.with_text_endpoints(self.text.chat_url(), self.text.memory_url());
        match &self.media {
            Some(media) => config.with_media_engine(media.media_url()),
            None => config,
        }
    }
}

impl std::fmt::Debug for RunningEngines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningEngines")
            .field("chat", &self.text.chat_url())
            .field("memory", &self.text.memory_url())
            .field("media", &self.media.as_ref().map(MediaEngine::media_url))
            .finish()
    }
}

pub async fn start_engines(settings: &Settings) -> Result<RunningEngines, String> {
    let text = start_text_engine().await?;

    let media = if settings.models.image.enabled {
        start_media_engine()
            .await
            .inspect_err(|err| log::warn!("media engine unavailable: {err}"))
            .ok()
    } else {
        None
    };

    Ok(RunningEngines { text, media })
}
