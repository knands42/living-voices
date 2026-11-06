//! Supervises the `llama-server` child process.
//!
//! A server is not a pipeline step: steps are one-shot and idempotent, while a
//! process is stateful and has to be stopped. It lives here instead, owning its
//! `Child` so nothing else can leak it.

use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::provision::helper::{llama_server_binary, provision_llama_path};

pub const PREFERRED_PORT: u16 = 11_435;
const PORT_SEARCH_LIMIT: u16 = 10;

/// Prompt plus reply has to fit: llama.cpp's own default is far too small.
const CONTEXT_SIZE: u16 = 8_192;

const READY_TIMEOUT: Duration = Duration::from_secs(180);
const READY_POLL: Duration = Duration::from_millis(500);

pub struct LlamaServer {
    child: Child,
    base_url: String,
}

impl LlamaServer {
    pub fn spawn(model: &Path) -> Result<Self, String> {
        Self::spawn_at(&llama_server_binary(), model)
    }

    pub fn spawn_at(binary: &Path, model: &Path) -> Result<Self, String> {
        if !binary.is_file() {
            return Err(format!(
                "no llama-server at {}: build the source tree first",
                binary.display()
            ));
        }

        if !model.is_file() {
            return Err(format!("no model file at {}", model.display()));
        }

        let port = first_free_port(PREFERRED_PORT).ok_or_else(|| {
            format!(
                "no free port in {PREFERRED_PORT}..{}",
                PREFERRED_PORT + PORT_SEARCH_LIMIT
            )
        })?;

        let log = provision_llama_path().join("llama-server.log");
        let stdout = log_file(&log)?;
        let stderr = log_file(&log)?;

        let child = Command::new(binary)
            .arg("--model")
            .arg(model)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--ctx-size")
            .arg(CONTEXT_SIZE.to_string())
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map_err(|err| format!("could not start {}: {err}", binary.display()))?;

        Ok(Self {
            child,
            base_url: format!("http://127.0.0.1:{port}"),
        })
    }

    pub async fn wait_until_ready(&self) -> Result<(), String> {
        let url = format!("{}/health", self.base_url);
        let deadline = tokio::time::Instant::now() + READY_TIMEOUT;

        while tokio::time::Instant::now() < deadline {
            if let Ok(response) = reqwest::get(&url).await
                && response.status().is_success()
            {
                return Ok(());
            }

            tokio::time::sleep(READY_POLL).await;
        }

        Err(format!(
            "llama-server never answered {url} (waited {READY_TIMEOUT:?}, see llama-server.log)"
        ))
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for LlamaServer {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn first_free_port(preferred: u16) -> Option<u16> {
    let last = preferred.saturating_add(PORT_SEARCH_LIMIT);

    (preferred..=last).find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
}

fn log_file(path: &Path) -> Result<Stdio, String> {
    if let Some(parent) = path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        return Err(format!("could not create {}: {err}", parent.display()));
    }

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(Stdio::from)
        .map_err(|err| format!("could not open {}: {err}", path.display()))
}
