use std::fs::File;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use living_voices::provision::unpack;

fn scratch(name: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("lv-unpack-{name}-{}-{stamp}", std::process::id()))
}

/// A `.tar.gz` shaped like the llama.cpp source archive: one folder with files inside.
fn write_tar_gz(archive: &Path) {
    let encoder = GzEncoder::new(
        File::create(archive).expect("create archive"),
        Compression::fast(),
    );
    let mut builder = tar::Builder::new(encoder);

    for (name, body, mode) in [
        ("llama-b1/llama-server", b"#!/bin/sh\n".as_slice(), 0o755),
        ("llama-b1/libllama.so", b"lib".as_slice(), 0o644),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(mode);
        header.set_cksum();
        builder
            .append_data(&mut header, name, body)
            .expect("append");
    }

    let encoder = builder.into_inner().expect("finish tar");
    encoder.finish().expect("finish gzip");
}

#[test]
fn unpack_reads_a_tar_gz_and_keeps_the_exec_bit() {
    // given a tarball with one top-level folder
    let base = scratch("tar");
    std::fs::create_dir_all(&base).expect("dirs");
    let archive = base.join("llama.tar.gz");
    write_tar_gz(&archive);

    let into = base.join("out");

    // when
    unpack(&archive, &into).expect("unpack tar.gz");

    // then
    let binary = into.join("llama-b1/llama-server");
    assert!(binary.is_file(), "binary missing");
    assert!(into.join("llama-b1/libllama.so").is_file(), "lib missing");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = std::fs::metadata(&binary)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "exec bit lost: {mode:o}");
    }

    std::fs::remove_dir_all(&base).ok();
}

#[test]
fn unpack_rejects_an_unknown_archive() {
    // given
    let base = scratch("unknown");
    std::fs::create_dir_all(&base).expect("dirs");
    let archive = base.join("llama.rar");
    std::fs::write(&archive, b"nope").expect("write");

    // when
    let result = unpack(&archive, &base.join("out"));

    // then
    let err = result.expect_err("should reject .rar");
    assert!(
        err.contains("llama.rar"),
        "error should name the file: {err}"
    );

    std::fs::remove_dir_all(&base).ok();
}
