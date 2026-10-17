use std::path::PathBuf;

pub struct TempDb(pub PathBuf);
impl Drop for TempDb {
    fn drop(&mut self) {
        let base = self.0.to_string_lossy().into_owned();
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{base}{suffix}"));
        }
    }
}
