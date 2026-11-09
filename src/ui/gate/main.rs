use std::sync::{Arc, Mutex};

use iced::{Element, Size, Task, Theme};

use super::view;
use crate::config::Config;
use crate::provision::entrypoint::start_provisioning;
use crate::provision::{ComfyEvent, Gpu, MediaRecommendation, human_bytes};
use crate::ui::task::on_tokio;

const PROGRESS_EVERY_BYTES: u64 = 64 * 1024 * 1024;

/// Runs on the first time the app is run.
pub fn gate_run(config: Config, gpu: &Gpu) -> Result<Config, iced::Error> {
    let gpu = *gpu;
    let fallback = config.clone();
    let outcome = Arc::new(Mutex::new(None));
    let slot = outcome.clone();

    iced::application(
        move || StateGate::boot(config.clone(), gpu, slot.clone()),
        StateGate::update,
        StateGate::view,
    )
    .title("Living Voices")
    .theme(|_: &StateGate| Theme::CatppuccinMacchiato)
    .window_size(Size::new(1360.0, 900.0))
    .antialiasing(true)
    .run()?;

    let resolved = outcome.lock().ok().and_then(|mut slot| slot.take());

    Ok(resolved.unwrap_or(fallback))
}

#[derive(Debug, Clone)]
pub enum MessageGate {
    MediaEngineToggled(bool),
    Confirmed,
    Provisioned(Option<Result<Config, String>>),
}

pub struct StateGate {
    config: Config,
    handle: tokio::runtime::Handle,
    outcome: Arc<Mutex<Option<Config>>>,
    recommendation: MediaRecommendation,
    gpu: Gpu,
    media_engine: bool,
    decision: Option<bool>,
}

impl StateGate {
    pub fn boot(config: Config, gpu: Gpu, outcome: Arc<Mutex<Option<Config>>>) -> Self {
        let recommendation = gpu.media_recommendation();

        Self {
            config,
            handle: tokio::runtime::Handle::current(),
            outcome,
            recommendation,
            gpu,
            media_engine: recommendation.suggests_midia_engine(),
            decision: None,
        }
    }

    pub fn update(&mut self, message: MessageGate) -> Task<MessageGate> {
        match message {
            MessageGate::MediaEngineToggled(enabled) => {
                self.media_engine = enabled;
                Task::none()
            }
            MessageGate::Confirmed => {
                self.decision = Some(self.media_engine);

                let config = self.config.clone();
                let handle = self.handle.clone();
                let media_engine = self.media_engine;

                Task::perform(
                    on_tokio(handle, async move {
                        let mut config = config;

                        if let Err(err) = config.set_media_engine(media_engine) {
                            log::warn!("could not write the answer into config.toml: {err}");
                        }

                        start_provisioning(
                            config,
                            on_text_engine_callback(),
                            on_media_engine_callback(),
                        )
                        .await
                    }),
                    MessageGate::Provisioned,
                )
            }
            MessageGate::Provisioned(result) => {
                match result {
                    Some(Ok(config)) => {
                        self.config = config.clone();

                        if let Ok(mut slot) = self.outcome.lock() {
                            *slot = Some(config);
                        }
                    }
                    Some(Err(err)) => log::warn!("provisioning failed: {err}"),
                    None => log::warn!("provisioning task was cancelled"),
                }

                iced::exit()
            }
        }
    }

    pub fn view(&self) -> Element<'_, MessageGate> {
        view::view(self.gpu, self.recommendation, self.media_engine)
    }

    /// What the player picked, `None` until they confirm.
    pub fn decision(&self) -> Option<bool> {
        self.decision
    }
}

pub fn on_text_engine_callback() -> impl FnMut(String, u64, Option<u64>) {
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
            None => log::info!("llama model: {}", human_bytes(written)),
        }
    }
}

pub fn on_media_engine_callback() -> impl FnMut(ComfyEvent) {
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
