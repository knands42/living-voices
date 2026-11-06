//! Supervises the `llama-server` child process.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::provision::helper::llama_server_binary;

pub const PREFERRED_PORT: u16 = 11_435;
const PORT_SEARCH_LIMIT: u16 = 10;

/// Prompt plus reply has to fit: llama.cpp's own default is far too small.
const CONTEXT_SIZE: u16 = 8_192;

const READY_TIMEOUT: Duration = Duration::from_secs(180);
const READY_POLL: Duration = Duration::from_millis(500);

pub struct LlamaServer {
    child: Child,
    base_url: String,
    exposed_port: u16,
}

impl LlamaServer {
    pub fn spawn(model: &Path, alias: &str) -> Result<Self, String> {
        let port = Self::get_available_port();
        Self::spawn_at(&llama_server_binary(), model, alias, port)
    }

    fn spawn_at(binary: &Path, model: &Path, alias: &str, port: u16) -> Result<Self, String> {
        if !binary.is_file() {
            return Err(format!(
                "no llama-server at {}: install it with `just provision-llama-install`",
                binary.display()
            ));
        }

        if !model.is_file() {
            return Err(format!("no model file at {}", model.display()));
        }

        let log = log_path(binary);
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
            .arg("--alias")
            .arg(alias)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map_err(|err| format!("could not start {}: {err}", binary.display()))?;

        Ok(Self {
            child,
            base_url: format!("http://127.0.0.1:{port}"),
            exposed_port: port,
        })
    }

    pub fn get_available_port() -> u16 {
        let last = PREFERRED_PORT.saturating_add(PORT_SEARCH_LIMIT);

        (PREFERRED_PORT..=last)
            .find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
            .ok_or_else(|| {
                format!(
                    "no free port in {PREFERRED_PORT}..{}",
                    PREFERRED_PORT + PORT_SEARCH_LIMIT
                )
            })
            .expect("no free port available")
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

/// The log belongs next to the binary, so a test run with a fake server never
/// writes into the real install.
fn log_path(binary: &Path) -> PathBuf {
    binary
        .parent()
        .map(|dir| dir.join("llama-server.log"))
        .unwrap_or_else(|| PathBuf::from("llama-server.log"))
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

mod test {
    use crate::provision::{LlamaServer, PREFERRED_PORT};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};

    fn scratch(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after the epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("lv-server-{name}-{}-{stamp}", std::process::id()))
    }

    #[cfg(unix)]
    fn fake_server(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(path, b"#!/bin/sh\nexec sleep 30\n").expect("write");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }

    #[test]
    fn spawn_reports_a_missing_binary() {
        // given a model but nothing built yet
        let base = scratch("missing-bin");
        std::fs::create_dir_all(&base).expect("dirs");
        let model = base.join("model.gguf");
        std::fs::write(&model, b"GGUF").expect("write");
        let port = LlamaServer::get_available_port();

        // when
        let result = LlamaServer::spawn_at(&base.join("llama-server"), &model, "test-model", port);

        // then
        let err = result.err().expect("should refuse");
        assert!(err.contains("llama-server"), "error should name it: {err}");

        std::fs::remove_dir_all(&base).ok();
    }

    #[cfg(unix)]
    #[test]
    fn spawn_refuses_a_missing_model() {
        // given a binary but no weights
        let base = scratch("missing-model");
        let binary = base.join("llama-server");
        let port = LlamaServer::get_available_port();
        fake_server(&binary);

        // when
        let result = LlamaServer::spawn_at(&binary, &base.join("model.gguf"), "test-model", port);

        // then
        let err = result.err().expect("should refuse");
        assert!(err.contains("model"), "error should name it: {err}");

        std::fs::remove_dir_all(&base).ok();
    }

    #[cfg(unix)]
    #[test]
    fn spawn_runs_and_stop_kills_the_child() {
        // given a binary that stays alive like a server would
        let base = scratch("lifecycle");
        let binary = base.join("llama-server");
        let port = LlamaServer::get_available_port();
        fake_server(&binary);

        let model = base.join("model.gguf");
        std::fs::write(&model, b"GGUF").expect("write");

        // when
        let mut server = LlamaServer::spawn_at(&binary, &model, "test-model", port).expect("spawn");

        // then it is up, on a port of its own
        assert!(server.is_running(), "child should be alive");
        assert!(
            server.base_url().starts_with("http://127.0.0.1:1143"),
            "unexpected base url: {}",
            server.base_url()
        );

        // when
        server.stop();

        // then
        assert!(!server.is_running(), "child should be dead");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn first_free_port_skips_a_taken_port() {
        // given a port somebody else already holds
        let taken =
            TcpListener::bind(format!("127.0.0.1:{}", PREFERRED_PORT)).expect("bind ephemeral");
        let taken_port = taken.local_addr().expect("addr").port();

        // when
        let next = LlamaServer::get_available_port();

        // then it moved on instead of answering with the busy one
        assert_ne!(next, taken_port);
        assert!(next > taken_port);
    }
}
