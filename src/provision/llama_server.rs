//! Supervises the `llama-server` child process.

use crate::provision::helper;
use crate::provision::helper::{llama_server_binary, log_file};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub const PREFERRED_PORT: u16 = 11_435;
const PORT_SEARCH_LIMIT: u16 = 10;

/// Prompt plus reply has to fit: llama.cpp's own default is far too small
const CONTEXT_SIZE: u16 = 8_192;

/// Offload every layer that fits in VRAM and keep the rest on the CPU
const GPU_LAYERS: &str = "auto";

/// Quantised KV cache roughly halves the context footprint, which is what lets
/// `CONTEXT_SIZE` fit an 8 GiB budget
const FLASH_ATTENTION: &str = "on";
const KV_CACHE_TYPE: &str = "q8_0";

const READY_TIMEOUT: Duration = Duration::from_secs(180);
const READY_POLL: Duration = Duration::from_millis(500);
const LLAMA_LOG: &str = "llama-server.log";

pub struct LlamaServer {
    child: Child,
    base_url: String,
    exposed_port: u16,
}

impl LlamaServer {
    pub fn spawn(model: &Path, alias: &str) -> Result<Self, String> {
        let port = helper::get_available_port(PREFERRED_PORT, PORT_SEARCH_LIMIT)?;
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
            .args(["--gpu-layers", GPU_LAYERS])
            .args(["--flash-attn", FLASH_ATTENTION])
            .args(["--cache-type-k", KV_CACHE_TYPE])
            .args(["--cache-type-v", KV_CACHE_TYPE])
            .arg("--jinja")
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

    pub async fn wait_until_ready(&self) -> Result<(), String> {
        helper::wait_until_ready(self.base_url(), READY_POLL, READY_TIMEOUT, LLAMA_LOG).await
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn stop(&mut self) {
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

#[cfg(test)]
mod test {
    use crate::provision::llama_server::PORT_SEARCH_LIMIT;
    use crate::provision::{LlamaServer, PREFERRED_PORT, helper};
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

    /// Same stand-in, but it spills the arguments it was given into `argv`.
    #[cfg(unix)]
    fn recording_server(path: &Path, argv: &Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nexec sleep 30\n",
            argv.display()
        );
        std::fs::write(path, script).expect("write");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }

    /// The child writes the file itself, so reading it needs to wait for the shell.
    #[cfg(unix)]
    fn wait_for_argv(argv: &Path) -> Vec<String> {
        for _ in 0..100 {
            if let Ok(text) = std::fs::read_to_string(argv)
                && !text.is_empty()
            {
                return text.lines().map(str::to_string).collect();
            }

            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        panic!("the fake server never recorded its arguments");
    }

    #[test]
    fn spawn_reports_a_missing_binary() {
        // given a model but nothing built yet
        let base = scratch("missing-bin");
        std::fs::create_dir_all(&base).expect("dirs");
        let model = base.join("model.gguf");
        std::fs::write(&model, b"GGUF").expect("write");
        let port =
            helper::get_available_port(PREFERRED_PORT, PORT_SEARCH_LIMIT).expect("free port");

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
        let port =
            helper::get_available_port(PREFERRED_PORT, PORT_SEARCH_LIMIT).expect("free port");
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
        let port =
            helper::get_available_port(PREFERRED_PORT, PORT_SEARCH_LIMIT).expect("free port");
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

    #[cfg(unix)]
    #[test]
    fn spawn_hands_the_engine_its_offload_and_cache_flags() {
        // given a binary that reports what it was called with
        let base = scratch("flags");
        let binary = base.join("llama-server");
        let argv = base.join("argv.txt");
        let port =
            helper::get_available_port(PREFERRED_PORT, PORT_SEARCH_LIMIT).expect("free port");
        recording_server(&binary, &argv);

        let model = base.join("model.gguf");
        std::fs::write(&model, b"GGUF").expect("write");

        // when
        let mut server = LlamaServer::spawn_at(&binary, &model, "test-model", port).expect("spawn");
        let recorded = wait_for_argv(&argv);

        // then the model, context and alias survive the refactor
        assert!(recorded.iter().any(|arg| arg == "8192"), "{recorded:?}");
        assert!(
            recorded.iter().any(|arg| arg == "test-model"),
            "{recorded:?}"
        );

        // and the GPU offload, Flash Attention, quantised cache and jinja are set
        assert_pair(&recorded, "--gpu-layers", "auto");
        assert_pair(&recorded, "--flash-attn", "on");
        assert_pair(&recorded, "--cache-type-k", "q8_0");
        assert_pair(&recorded, "--cache-type-v", "q8_0");
        assert!(recorded.iter().any(|arg| arg == "--jinja"), "{recorded:?}");

        // when
        server.stop();

        std::fs::remove_dir_all(&base).ok();
    }

    #[cfg(unix)]
    fn assert_pair(argv: &[String], flag: &str, value: &str) {
        let at = argv
            .iter()
            .position(|arg| arg == flag)
            .unwrap_or_else(|| panic!("missing {flag} in {argv:?}"));

        assert_eq!(
            argv.get(at + 1).map(String::as_str),
            Some(value),
            "{flag} should be followed by {value}: {argv:?}"
        );
    }
}
