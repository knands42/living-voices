use super::comfy::layout::{comfy_bundle, comfy_dir, comfy_main};
use super::llama::layout::{MEMORY_MODEL_ALIAS, MODEL_ALIAS};
use crate::config::Config;
use crate::provision::comfy::comfy_server::ComfyServer;
use crate::provision::comfy::setup;
use crate::provision::llama::llama_server::{CHAT_PORT, EngineProfile, LlamaServer, MEMORY_PORT};
use crate::provision::llama::model;
use crate::provision::{ComfyEvent, extract};

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

pub async fn start_provisioning<T, M>(
    config: Config,
    mut text_engine_callback: T,
    mut media_engine_callback: M,
) -> Result<Config, String>
where
    T: FnMut(String, u64, Option<u64>) + Send + 'static,
    M: FnMut(ComfyEvent) + Send + 'static,
{
    let engine = match start_text_engine(&mut text_engine_callback).await {
        Ok(engine) => {
            log::info!("chat engine answering on {}", engine.chat_url());
            log::info!("memory engine answering on {}", engine.memory_url());
            Some(engine)
        }
        Err(err) => {
            log::warn!("text engine unavailable: {err}");
            None
        }
    };

    let config = match &engine {
        Some(engine) => config.with_text_endpoints(engine.chat_url(), engine.memory_url()),
        None => {
            log::warn!("using configured text endpoints instead of the engine");
            config
        }
    };
    config.persist_to_file().map_err(|err| err.to_string())?;

    let mut config = if config.settings.models.image.enabled {
        let media_engine = match start_media_engine(&mut media_engine_callback).await {
            Ok(engine) => {
                log::info!("media engine answering on {}", engine.base_url());
                Some(engine)
            }
            Err(err) => {
                log::warn!("media engine unavailable: {err}");
                None
            }
        };

        match media_engine {
            Some(engine) => config.with_media_engine(engine.base_url()),
            None => {
                log::warn!("using the configured image endpoint instead of the engine");
                config
            }
        }
    } else {
        config
    };

    config.settings.setup.prompt_gate_setup();
    config.persist_to_file().map_err(|err| err.to_string())?;

    Ok(config)
}

/// Download both models if needed, start one `llama-server` per role, wait until both answer.
async fn start_text_engine<F>(on_progress: &mut F) -> Result<TextEngine, String>
where
    F: FnMut(String, u64, Option<u64>),
{
    let chat_model = model::ensure_chat(&mut *on_progress).await?;
    let memory_model = model::ensure_memory(&mut *on_progress).await?;

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

/// First run: extract the shipped bundle, build the venv from the lock, fetch
/// the image model, start ComfyUI, wait until it answers.
async fn start_media_engine<M>(media_engine_callback: &mut M) -> Result<ComfyServer, String>
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

    let server = ComfyServer::spawn()?;
    server.wait_until_ready().await?;

    log::info!("media engine answering on {}", server.base_url());

    Ok(server)
}
