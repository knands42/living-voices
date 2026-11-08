// #![allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction)]

use crate::app::DefaultContext;
use crate::ui::callbacks::register::register_callbacks;

pub mod callbacks;
pub mod notice;
pub mod viewmodels;
mod window;

slint::include_modules!();

pub fn run(ctx: DefaultContext) -> Result<(), slint::PlatformError> {
    let ui = RootWindow::new()?;

    let summaries = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(ctx.adventures.get_adventures_summaries())
    })
    .unwrap_or_else(|err| {
        log::error!("Could not load adventure summaries: {err}");
        Vec::new()
    });

    window::fill_root_window_inputs(&ui, &summaries);
    register_callbacks(&ui, &ctx);

    ui.run()
}
