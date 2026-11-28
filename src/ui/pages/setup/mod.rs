mod header;
mod question;
mod view;
mod working;

use std::sync::Arc;

use crate::config::{ConfigHandle, Settings};
use crate::provision::{
    ComfyEvent, Gpu, MediaRecommendation, RunningEngines, human_bytes, start_engines,
    start_provisioning,
};
use crate::ui::pages::setup::header::header;
use crate::ui::pages::setup::question::question;
use crate::ui::pages::setup::view::{specs, verdict};
use crate::ui::pages::setup::working::working;
use crate::ui::pages::{Event, Page};
use crate::ui::task::on_tokio;
use crate::ui::theme;
use iced::futures::SinkExt;
use iced::futures::channel::mpsc::Sender;
use iced::widget::{container, scrollable};
use iced::{Alignment, Element, Length, Task};

const PROGRESS_EVERY_BYTES: u64 = 64 * 1024 * 1024;
const COLUMN_LENGTH: f32 = 620.0;

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
    Tick,
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

impl Progress {
    pub fn label(&self) -> String {
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

    pub fn ratio(&self) -> f32 {
        match self {
            Self::Model {
                written,
                total: Some(total),
                ..
            } if *total > 0 => (*written as f32 / *total as f32).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }

    pub fn percent(&self) -> Option<i32> {
        match self {
            Self::Model {
                total: Some(total), ..
            } if *total > 0 => Some((self.ratio() * 100.0).round() as i32),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Phase {
    Asking,
    Installing,
    Starting,
    Running,
    Failed(String),
}

pub struct State {
    config: ConfigHandle,
    gpu: Gpu,
    handle: tokio::runtime::Handle,
    recommendation: MediaRecommendation,
    media_engine: bool,
    phase: Phase,
    progress: Option<Progress>,
    notice: Option<&'static str>,
    spinner: f32,
}

impl State {
    pub fn boot(
        config: ConfigHandle,
        gpu: Gpu,
        handle: tokio::runtime::Handle,
    ) -> (Self, Task<Message>) {
        let recommendation = gpu.media_recommendation();
        let media_engine = if config.needs_setup() {
            recommendation.suggests_midia_engine()
        } else {
            config.settings().models.image.enabled
        };

        let mut state = Self {
            config,
            gpu,
            handle,
            recommendation,
            media_engine,
            phase: Phase::Asking,
            progress: None,
            notice: None,
            spinner: 0.0,
        };

        // A first run waits for the player's answer; any other run starts the engines now.
        let task = if state.needs_answer() {
            Task::none()
        } else {
            state.start()
        };

        (state, task)
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
            Message::BackRequested => return (Task::none(), Event::Show(Page::MainHub)),
            Message::Progress(progress) => self.progress = Some(progress),
            Message::Provisioned => {
                self.phase = Phase::Starting;
                self.progress = None;
            }
            Message::EnginesStarted(engines) => {
                self.phase = Phase::Running;
                engines.stamp(&self.config);
                return (
                    Task::none(),
                    Event::EnginesReady {
                        engines,
                        config: self.config.clone(),
                    },
                );
            }
            Message::Failed(err) => {
                log::warn!("engines could not start: {err}");
                self.phase = Phase::Failed(err);
            }
            Message::Tick => {
                self.spinner = (self.spinner + 0.12) % std::f32::consts::TAU;
            }
        }

        (Task::none(), Event::None)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body = match &self.phase {
            Phase::Installing => working(
                "Installing the engines",
                0,
                self.progress.as_ref(),
                self.spinner,
            ),
            Phase::Starting => working("Starting the engines", 1, None, self.spinner),
            Phase::Asking | Phase::Running | Phase::Failed(_) => question(self),
        };

        let centre = iced::widget::column![
            specs(self.gpu, self.recommendation),
            verdict(self.recommendation),
            body,
        ]
        .spacing(14.0)
        .width(Length::Fill);

        // Scrolled, because a launcher window is often shorter than the page.
        let scrolled = scrollable(
            container(container(centre).width(Length::Fixed(COLUMN_LENGTH)))
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
        .height(Length::Fill)
        .width(Length::Fill);

        container(
            iced::widget::column![header(self), scrolled]
                .spacing(14.0)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding([18.0, 22.0])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::page)
        .into()
    }

    pub fn needs_answer(&self) -> bool {
        self.config.needs_setup()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        match self.phase {
            Phase::Installing | Phase::Starting => iced::Subscription::run(ticker_stream),
            _ => iced::Subscription::none(),
        }
    }

    fn start(&mut self) -> Task<Message> {
        self.phase = Phase::Installing;
        self.progress = None;
        bring_up(self.handle.clone(), self.config.settings())
    }
}

fn ticker_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, move |mut output: Sender<Message>| async move {
        loop {
            futures_timer::Delay::new(std::time::Duration::from_millis(16)).await;
            if output.send(Message::Tick).await.is_err() {
                break;
            }
        }
    })
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
