use std::future::Future;

/// Runs a manager future on the tokio runtime `main` owns.
///
/// `Task::perform` is driven by iced's own executor, where `sqlx` and `reqwest`
/// cannot find the reactor they need ("this functionality requires a Tokio
/// context"). Spawning back through a captured handle — grabbed in `boot`, which
/// still runs inside `#[tokio::main]` — puts the work on tokio's threads.
pub fn on_tokio<T: Send + 'static>(
    handle: tokio::runtime::Handle,
    future: impl Future<Output = T> + Send + 'static,
) -> impl Future<Output = Option<T>> {
    async move {
        match handle.spawn(future).await {
            Ok(value) => Some(value),
            Err(err) => {
                log::error!("background task failed: {err}");
                None
            }
        }
    }
}
