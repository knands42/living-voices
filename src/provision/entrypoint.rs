use super::helper::human_bytes;
use super::model;
use super::server::LlamaServer;

const PROGRESS_EVERY_BYTES: u64 = 64 * 1024 * 1024;

/// Download the model if needed, start `llama-server` on it, wait until it answers.
pub async fn start_text_engine() -> Result<LlamaServer, String> {
    let mut last_logged = 0;

    let model_path = model::ensure(|written, total| {
        if written.saturating_sub(last_logged) < PROGRESS_EVERY_BYTES {
            return;
        }

        last_logged = written;
        match total {
            Some(total) => log::info!("model: {} / {}", human_bytes(written), human_bytes(total)),
            None => log::info!("model: {}", human_bytes(written)),
        }
    })
    .await?;

    log::info!("model ready at {}", model_path.display());

    let server = LlamaServer::spawn(&model_path, model::MODEL_ALIAS)?;
    server.wait_until_ready().await?;

    log::info!("text engine answering on {}", server.base_url());

    Ok(server)
}
