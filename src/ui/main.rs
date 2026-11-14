use iced::{Size, Theme};

use crate::app::AppContext;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort, PromptPort};
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::config::Config;
use crate::provision::Gpu;
use crate::ui::app::{App, Rebuild};

pub fn run<
    C: ChatPort + Clone + 'static,
    N: NarratorPort + Clone + 'static,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
    P: PromptPort + Clone + 'static,
    I: ImagePort + Clone + 'static,
>(
    ctx: AppContext<C, N, M, A, P, I>,
    rebuild: impl Fn(&Config) -> Result<AppContext<C, N, M, A, P, I>, String> + Send + Sync + 'static,
    config: Config,
    gpu: Gpu,
    open_setup: bool,
) -> iced::Result {
    let rebuild: Rebuild<C, N, M, A, P, I> = std::sync::Arc::new(rebuild);

    iced::application(
        move || {
            App::boot(
                ctx.clone(),
                rebuild.clone(),
                config.clone(),
                gpu,
                open_setup,
            )
        },
        App::update,
        App::view,
    )
    .title("Living Voices")
    .theme(|_: &App<C, N, M, A, P, I>| Theme::CatppuccinMacchiato)
    .window_size(Size::new(1360.0, 900.0))
    .antialiasing(true)
    .run()
}
