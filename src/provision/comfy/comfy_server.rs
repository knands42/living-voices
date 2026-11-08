use crate::provision::helper;
use crate::provision::helper::{COMFY_HOST, comfy_dir, comfy_venv_python, log_file};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub const PREFERRED_PORT: u16 = 8_189;
const PORT_SEARCH_LIMIT: u16 = 10;

const READY_TIMEOUT: Duration = Duration::from_secs(180);
const READY_POLL: Duration = Duration::from_millis(500);
const COMFY_LOG: &str = "comfy_ui.log";

pub struct ComfyServer {
    child: Child,
    base_url: String,
}

impl ComfyServer {
    pub fn spawn() -> Result<Self, String> {
        let port = helper::get_available_port(PREFERRED_PORT, PORT_SEARCH_LIMIT)?;
        Self::spawn_at(&comfy_venv_python(), &comfy_dir(), COMFY_HOST, port)
    }

    fn spawn_at(python: &Path, dir: &Path, host: &str, port: u16) -> Result<Self, String> {
        if !python.is_file() {
            return Err(format!(
                "no comfy venv at {}: install deps first",
                python.display()
            ));
        }

        if !dir.join("main.py").is_file() {
            return Err(format!("no main.py under {}", dir.display()));
        }

        let log = dir.join(COMFY_LOG);
        let stdout = log_file(&log)?;
        let stderr = log_file(&log)?;

        let child = Command::new(python)
            .arg("main.py")
            .arg("--listen")
            .arg(host)
            .arg("--port")
            .arg(port.to_string())
            .arg("--disable-auto-launch")
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map_err(|err| format!("could not start comfy: {err}"))?;

        Ok(Self {
            child,
            base_url: format!("http://{host}:{port}"),
        })
    }

    pub async fn wait_until_ready(&self) -> Result<(), String> {
        helper::wait_until_ready(self.base_url(), READY_POLL, READY_TIMEOUT, COMFY_LOG).await
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

impl Drop for ComfyServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after the epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("lv-comfy-{name}-{}-{stamp}", std::process::id()))
    }

    #[cfg(unix)]
    fn fake_python(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(path, b"#!/bin/sh\nexec sleep 30\n").expect("write");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }

    #[test]
    fn spawn_refuses_a_missing_venv() {
        // given a tree without a venv python
        let base = scratch("missing-venv");
        std::fs::create_dir_all(&base).expect("dirs");
        std::fs::write(base.join("main.py"), b"").expect("main");

        // when
        let result = ComfyServer::spawn_at(&base.join("venv/bin/python"), &base, "127.0.0.1", 8188);

        // then
        let err = result.err().expect("should refuse");
        assert!(err.contains("venv"), "error should name it: {err}");

        std::fs::remove_dir_all(&base).ok();
    }

    #[cfg(unix)]
    #[test]
    fn spawn_refuses_a_tree_without_main() {
        // given a venv python but no main.py
        let base = scratch("missing-main");
        let python = base.join("venv/bin/python");
        fake_python(&python);

        // when
        let result = ComfyServer::spawn_at(&python, &base, "127.0.0.1", 8188);

        // then
        let err = result.err().expect("should refuse");
        assert!(err.contains("main.py"), "error should name it: {err}");

        std::fs::remove_dir_all(&base).ok();
    }

    #[cfg(unix)]
    #[test]
    fn spawn_runs_and_stop_kills_the_child() {
        // given a venv python that stays alive like the engine would
        let base = scratch("lifecycle");
        let python = base.join("venv/bin/python");
        fake_python(&python);
        std::fs::write(base.join("main.py"), b"").expect("main");

        // when
        let mut server = ComfyServer::spawn_at(&python, &base, "127.0.0.1", 8188).expect("spawn");

        // then it is up on the comfy url
        assert!(server.is_running(), "child should be alive");
        assert_eq!(server.base_url(), "http://127.0.0.1:8188");

        // when
        server.stop();

        // then
        assert!(!server.is_running(), "child should be dead");

        std::fs::remove_dir_all(&base).ok();
    }
}
