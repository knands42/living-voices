//! Streams a `.tar.zst` into a directory: the shipped ComfyUI bundle is
//! unpacked once, on first run, with no intermediate tarball on disk.

use std::fs::File;
use std::path::Path;

use tar::Archive;
use zstd::Decoder;

pub fn tar_zst(archive: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive)
        .map_err(|err| format!("could not open {}: {err}", archive.display()))?;

    let decoder =
        Decoder::new(file).map_err(|err| format!("could not read {}: {err}", archive.display()))?;

    std::fs::create_dir_all(destination)
        .map_err(|err| format!("could not create {}: {err}", destination.display()))?;

    Archive::new(decoder)
        .unpack(destination)
        .map_err(|err| format!("could not unpack into {}: {err}", destination.display()))
}

#[cfg(test)]
mod test {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after the epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("lv-extract-{name}-{}-{stamp}", std::process::id()))
    }

    fn write_tar_zst(archive: &Path, entry: &str, body: &[u8]) {
        let file = File::create(archive).expect("create archive");
        let encoder = zstd::Encoder::new(file, 0).expect("encoder").auto_finish();

        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, entry, body)
            .expect("append");
        builder.into_inner().expect("finish tar");
    }

    #[test]
    fn unpacks_a_tar_zst_into_the_destination() {
        // given a tar.zst holding one file
        let base = scratch("round-trip");
        std::fs::create_dir_all(&base).expect("dirs");
        let archive = base.join("bundle.tar.zst");
        write_tar_zst(&archive, "main.py", b"print('comfy')\n");

        // when it is extracted
        let dest = base.join("comfyui");
        tar_zst(&archive, &dest).expect("extract");

        // then the file lands with its contents intact
        let unpacked = dest.join("main.py");
        assert!(unpacked.is_file(), "main.py should exist at {unpacked:?}");
        assert_eq!(
            std::fs::read_to_string(&unpacked).expect("read"),
            "print('comfy')\n"
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn reports_a_missing_archive() {
        // given no archive on disk
        let base = scratch("missing");

        // when
        let result = tar_zst(&base.join("nope.tar.zst"), &base.join("out"));

        // then it names what it could not open
        let err = result.expect_err("should fail");
        assert!(err.contains("nope.tar.zst"), "error should name it: {err}");
    }
}
