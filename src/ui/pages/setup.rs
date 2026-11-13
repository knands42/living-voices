use std::sync::Arc;

use iced::futures::channel::mpsc::Sender;
use iced::widget::{Space, button, column, container, progress_bar, row, text, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use crate::config::{Config, Settings};
use crate::provision::entrypoint::start_provisioning;
use crate::provision::{
    ComfyEvent, Gpu, MediaRecommendation, RunningEngines, human_bytes, start_engines,
};
use crate::ui::task::on_tokio;

const PROGRESS_EVERY_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub enum Message {
    MediaToggled(bool),
    Confirmed,
    BackRequested,
    Progress(Progress),
    Provisioned,
    // `Arc` because a message must be `Clone` and the server handles are not.
    EnginesStarted(Arc<RunningEngines>),
    Failed(String),
}

pub enum Event {
    None,
    BackRequested,
    EnginesReady {
        engines: Arc<RunningEngines>,
        config: Config,
    },
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

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Asking,
    Installing,
    Starting,
    Running,
    Failed(String),
}

pub struct SetupState {
    config: Config,
    gpu: Gpu,
    handle: tokio::runtime::Handle,
    recommendation: MediaRecommendation,
    media_engine: bool,
    phase: Phase,
    progress: Option<Progress>,
    notice: Option<&'static str>,
}

impl SetupState {
    pub fn new(config: Config, gpu: Gpu, handle: tokio::runtime::Handle) -> Self {
        let recommendation = gpu.media_recommendation();
        let media_engine = if config.needs_setup() {
            recommendation.suggests_midia_engine()
        } else {
            config.settings.models.image.enabled
        };

        Self {
            config,
            gpu,
            handle,
            recommendation,
            media_engine,
            phase: Phase::Asking,
            progress: None,
            notice: None,
        }
    }

    pub fn needs_answer(&self) -> bool {
        self.config.needs_setup()
    }

    pub fn start(&mut self) -> Task<Message> {
        self.phase = Phase::Installing;
        self.progress = None;
        bring_up(self.handle.clone(), self.config.settings.clone())
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::MediaToggled(enabled) => {
                self.media_engine = enabled;
                self.notice = None;
            }
            Message::Confirmed => {
                if let Err(err) = self.config.set_custom_config(self.media_engine) {
                    log::warn!("could not write the answer into config.toml: {err}");
                }

                if self.phase == Phase::Running {
                    self.notice = Some("Saved. The change applies on the next launch.");
                    return (Task::none(), Event::None);
                }

                return (self.start(), Event::None);
            }
            Message::BackRequested => return (Task::none(), Event::BackRequested),
            Message::Progress(progress) => self.progress = Some(progress),
            Message::Provisioned => {
                self.phase = Phase::Starting;
                self.progress = None;
            }
            Message::EnginesStarted(engines) => {
                self.phase = Phase::Running;
                let config = engines.stamp(self.config.clone());
                return (Task::none(), Event::EnginesReady { engines, config });
            }
            Message::Failed(err) => {
                log::warn!("engines could not start: {err}");
                self.phase = Phase::Failed(err);
            }
        }

        (Task::none(), Event::None)
    }
}

fn bring_up(handle: tokio::runtime::Handle, settings: Settings) -> Task<Message> {
    Task::run(
        iced::stream::channel(256, move |output: Sender<Message>| async move {
            let mut text_output = output.clone();
            let mut media_output = output.clone();
            let mut done_output = output;

            let mut last_text = 0;
            let text_callback = move |file: String, written: u64, total: Option<u64>| {
                if Some(written) != total
                    && written.saturating_sub(last_text) < PROGRESS_EVERY_BYTES
                {
                    return;
                }
                last_text = written;
                let _ = text_output.try_send(Message::Progress(Progress::Model {
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
                let _ = media_output.try_send(Message::Progress(progress));
            };

            let provision_settings = settings.clone();
            let provisioned = on_tokio(handle.clone(), async move {
                start_provisioning(&provision_settings, text_callback, media_callback).await
            })
            .await
            .unwrap_or_else(|| Err("install task was cancelled".to_string()));

            if let Err(err) = provisioned {
                let _ = done_output.try_send(Message::Failed(err));
                return;
            }
            let _ = done_output.try_send(Message::Provisioned);

            let started = on_tokio(handle, async move { start_engines(&settings).await })
                .await
                .unwrap_or_else(|| Err("engine start was cancelled".to_string()));

            let _ = done_output.try_send(match started {
                Ok(engines) => Message::EnginesStarted(Arc::new(engines)),
                Err(err) => Message::Failed(err),
            });
        }),
        |message| message,
    )
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
            Self::Model {
                written,
                total: Some(total),
                ..
            } if *total > 0 => (*written as f32 / *total as f32).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
}

pub fn view(setup: &SetupState) -> Element<'_, Message> {
    let body = match &setup.phase {
        Phase::Installing => working("Installing the engines", setup.progress.as_ref()),
        Phase::Starting => working("Starting the engines", None),
        Phase::Asking | Phase::Running | Phase::Failed(_) => question(setup),
    };

    container(
        column![
            row![
                text("Setup").size(28),
                Space::new().width(Length::Fill),
                button("Back to adventures").on_press(Message::BackRequested),
            ]
            .align_y(Alignment::Center),
            text("Before the story starts, a word about images").size(16),
            specs(setup.gpu, setup.recommendation),
            text(headline(setup.recommendation)).color(tone(setup.recommendation)),
            text(advice(setup.recommendation)).size(14),
            body,
        ]
        .spacing(16),
    )
    .style(container::rounded_box)
    .padding(28)
    .width(Length::Fill)
    .center_x(560)
    .into()
}

fn question(setup: &SetupState) -> Element<'_, Message> {
    let footer = match (&setup.phase, setup.notice) {
        (_, Some(notice)) => notice.to_string(),
        (Phase::Failed(err), None) => format!("The engines could not start: {err}"),
        (Phase::Running, None) => "The engines are running.".to_string(),
        _ => "You can come back to this page any time.".to_string(),
    };

    column![
        row![
            toggler(setup.media_engine)
                .label("Media engine (ComfyUI)")
                .on_toggle(Message::MediaToggled),
            Space::new().width(Length::Fill),
            button(text(if setup.media_engine {
                "Use images"
            } else {
                "Go without images"
            }))
            .style(button::primary)
            .on_press(Message::Confirmed),
        ]
        .align_y(Alignment::Center)
        .spacing(16),
        text(footer).size(12),
    ]
    .spacing(12)
    .into()
}

fn working<'a>(title: &'a str, progress: Option<&Progress>) -> Element<'a, Message> {
    let label = progress.map(Progress::label).unwrap_or_default();
    let ratio = progress.map(Progress::ratio).unwrap_or_default();

    column![
        text(title).size(16),
        progress_bar(0.0..=1.0, ratio),
        text(label).size(12),
    ]
    .spacing(8)
    .into()
}

fn specs(gpu: Gpu, recommendation: MediaRecommendation) -> Element<'static, Message> {
    container(
        column![
            row![
                text("Graphics card").size(14),
                Space::new().width(Length::Fill),
                text(gpu.card_label()).size(14),
            ],
            row![
                text("Video memory").size(14),
                Space::new().width(Length::Fill),
                text(gpu.vram_label()).size(14).color(tone(recommendation)),
            ],
        ]
        .spacing(8),
    )
    .style(container::bordered_box)
    .padding(14)
    .width(Length::Fill)
    .into()
}

fn tone(recommendation: MediaRecommendation) -> Color {
    match recommendation {
        MediaRecommendation::Comfortable => Color::from_rgb(0.42, 0.71, 0.42),
        MediaRecommendation::Tight => Color::from_rgb(1.0, 0.62, 0.25),
        MediaRecommendation::Limited => Color::from_rgb(0.75, 0.37, 0.31),
    }
}

fn headline(recommendation: MediaRecommendation) -> &'static str {
    match recommendation {
        MediaRecommendation::Comfortable => "Plenty of room for images.",
        MediaRecommendation::Tight => "Images will fit, with little headroom.",
        MediaRecommendation::Limited => "Images will not run well on this machine.",
    }
}

fn advice(recommendation: MediaRecommendation) -> &'static str {
    match recommendation {
        MediaRecommendation::Comfortable => {
            "The card carries the full-quality model alongside the text engine, so scenes and \
             portraits render while you keep talking."
        }
        MediaRecommendation::Tight => {
            "Eight to twelve gigabytes fit the lighter model only. Shots take longer, the card stays \
             warm, and the first run downloads the weights before anything draws. Leave it off and \
             the story still works exactly the same."
        }
        MediaRecommendation::Limited => {
            "No card with enough memory was found, so rendering would fall back to the processor or \
             fail outright. Everything except generated art works without it."
        }
    }
}
