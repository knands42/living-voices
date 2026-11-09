use std::sync::{Arc, Mutex};

use iced::futures::channel::mpsc::Sender;
use iced::{Element, Size, Task, Theme};

use super::view;
use crate::config::Config;
use crate::provision::entrypoint::{Engines, start_provisioning};
use crate::provision::{ComfyEvent, Gpu, MediaRecommendation, human_bytes};
use crate::ui::task::on_tokio;

const PROGRESS_EVERY_BYTES: u64 = 64 * 1024 * 1024;

/// What the gate hands back: the config it wrote the intent into, and the engines
/// this run started.
pub struct GateOutcome {
    pub config: Config,
    pub engines: Option<Engines>,
}

#[derive(Debug, Clone)]
pub enum MessageGate {
    MediaEngineToggled(bool),
    Confirmed(bool),
    Progress(Progress),
    Provisioned,
}

#[derive(Debug, Clone)]
pub enum Progress {
    InstallingDeps,
    Model {
        file: String,
        written: u64,
        total: Option<u64>,
    },
}

/// What the card shows while the engines come up.
pub struct BootProgress {
    pub label: String,
    pub ratio: f32,
}

impl Progress {
    fn label(&self) -> String {
        match self {
            Self::InstallingDeps => "Installing ComfyUI dependencies".to_string(),
            Self::Model {
                file,
                written,
                total: Some(total),
            } if *total > 0 => format!(
                "{file}: {} / {}",
                human_bytes(*written),
                human_bytes(*total)
            ),
            Self::Model { file, written, .. } => format!("{file}: {}", human_bytes(*written)),
        }
    }

    fn ratio(&self) -> f32 {
        match self {
            Self::InstallingDeps => 0.0,
            Self::Model {
                written,
                total: Some(total),
                ..
            } if *total > 0 => (*written as f32 / *total as f32).clamp(0.0, 1.0),
            Self::Model { .. } => 0.0,
        }
    }
}

/// Runs the first-run gate as its own window and returns what it decided and
/// started, or `None` when the player closed it without answering.
pub fn gate_run(config: Config, gpu: &Gpu) -> Result<Option<GateOutcome>, iced::Error> {
    let gpu = *gpu;
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

    Ok(outcome.lock().ok().and_then(|mut slot| slot.take()))
}

pub struct StateGate {
    config: Config,
    outcome: Arc<Mutex<Option<GateOutcome>>>,
    handle: tokio::runtime::Handle,
    recommendation: MediaRecommendation,
    gpu: Gpu,
    media_engine: bool,
    booting: bool,
    progress: Option<BootProgress>,
}

impl StateGate {
    pub fn boot(config: Config, gpu: Gpu, outcome: Arc<Mutex<Option<GateOutcome>>>) -> Self {
        let recommendation = gpu.media_recommendation();

        Self {
            config,
            outcome,
            handle: tokio::runtime::Handle::current(),
            recommendation,
            gpu,
            media_engine: recommendation.suggests_midia_engine(),
            booting: false,
            progress: None,
        }
    }

    pub fn update(&mut self, message: MessageGate) -> Task<MessageGate> {
        match message {
            MessageGate::MediaEngineToggled(enabled) => {
                self.media_engine = enabled;
                Task::none()
            }
            MessageGate::Confirmed(enabled) => {
                self.media_engine = enabled;
                self.booting = true;
                self.progress = Some(BootProgress {
                    label: "Starting the text engine".to_string(),
                    ratio: 0.0,
                });

                if let Err(err) = self.config.set_media_engine(enabled) {
                    log::warn!("could not write the answer into config.toml: {err}");
                }

                self.start()
            }
            MessageGate::Progress(progress) => {
                self.progress = Some(BootProgress {
                    label: progress.label(),
                    ratio: progress.ratio(),
                });

                Task::none()
            }
            MessageGate::Provisioned => iced::exit(),
        }
    }

    pub fn view(&self) -> Element<'_, MessageGate> {
        view::view(
            self.gpu,
            self.recommendation,
            self.media_engine,
            self.booting,
            self.progress.as_ref(),
        )
    }

    fn start(&self) -> Task<MessageGate> {
        let settings = self.config.settings.clone();
        let config = self.config.clone();
        let handle = self.handle.clone();
        let outcome = self.outcome.clone();

        Task::run(
            iced::stream::channel(256, move |output: Sender<MessageGate>| async move {
                let mut text_output = output.clone();
                let mut media_output = output.clone();
                let mut done_output = output.clone();

                let mut last_text = 0;
                let text_callback = move |file: String, written: u64, total: Option<u64>| {
                    if Some(written) != total
                        && written.saturating_sub(last_text) < PROGRESS_EVERY_BYTES
                    {
                        return;
                    }

                    last_text = written;
                    let _ = text_output.try_send(MessageGate::Progress(Progress::Model {
                        file,
                        written,
                        total,
                    }));
                };

                let mut last_media = 0;
                let media_callback = move |event: ComfyEvent| {
                    let progress = match event {
                        ComfyEvent::InstallingDeps => Progress::InstallingDeps,
                        ComfyEvent::InstallingModel {
                            file,
                            written,
                            total,
                        } => {
                            if Some(written) != total
                                && written.saturating_sub(last_media) < PROGRESS_EVERY_BYTES
                            {
                                return;
                            }

                            last_media = written;
                            Progress::Model {
                                file,
                                written,
                                total,
                            }
                        }
                    };

                    let _ = media_output.try_send(MessageGate::Progress(progress));
                };

                let engines = on_tokio(handle, async move {
                    start_provisioning(&settings, text_callback, media_callback).await
                })
                .await
                .unwrap_or_else(|| Err("provisioning task was cancelled".to_string()))
                .ok();

                if let Ok(mut slot) = outcome.lock() {
                    *slot = Some(GateOutcome { config, engines });
                }

                let _ = done_output.try_send(MessageGate::Provisioned);
            }),
            |message| message,
        )
    }
}
