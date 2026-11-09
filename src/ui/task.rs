use std::future::Future;

/// Runs a manager future on the tokio runtime `main` owns.
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
