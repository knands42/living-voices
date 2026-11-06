use std::net::TcpListener;
use std::path::{Path, PathBuf};

use living_voices::provision::LlamaServer;
use living_voices::provision::first_free_port;

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
fn first_free_port_skips_a_taken_port() {
    // given a port somebody else already holds
    let taken = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let taken_port = taken.local_addr().expect("addr").port();

    // when
    let next = first_free_port(taken_port).expect("a free port");

    // then it moved on instead of answering with the busy one
    assert_ne!(next, taken_port);
    assert!(next > taken_port);
}

#[test]
fn spawn_reports_a_missing_binary() {
    // given a model but nothing built yet
    let base = scratch("missing-bin");
    std::fs::create_dir_all(&base).expect("dirs");
    let model = base.join("model.gguf");
    std::fs::write(&model, b"GGUF").expect("write");

    // when
    let result = LlamaServer::spawn_at(&base.join("llama-server"), &model);

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
    fake_server(&binary);

    // when
    let result = LlamaServer::spawn_at(&binary, &base.join("model.gguf"));

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
    fake_server(&binary);

    let model = base.join("model.gguf");
    std::fs::write(&model, b"GGUF").expect("write");

    // when
    let mut server = LlamaServer::spawn_at(&binary, &model).expect("spawn");

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
