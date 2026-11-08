use std::net::TcpListener;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

/// Sizes for progress lines: `0 B`, `1023 B`, `1.5 KiB`, `6.0 GiB`.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub async fn wait_until_ready(
    url: &str,
    ready_pool: Duration,
    ready_timeout: Duration,
    log_file_name: &str,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + ready_timeout;

    while tokio::time::Instant::now() < deadline {
        if let Ok(response) = reqwest::get(url).await
            && response.status().is_success()
        {
            return Ok(());
        }

        tokio::time::sleep(ready_pool).await;
    }

    Err(format!(
        "never answered {url} (waited {ready_timeout:?}, see {log_file_name})"
    ))
}

pub(crate) fn get_available_port(
    preferred_port: u16,
    port_search_limit: u16,
) -> Result<u16, String> {
    let last = preferred_port.saturating_add(port_search_limit);

    (preferred_port..=last)
        .find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .ok_or_else(|| format!("no free port in {preferred_port}..={last}"))
}

pub fn log_file(path: &Path) -> Result<Stdio, String> {
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

#[cfg(test)]
mod test {
    use super::get_available_port;
    use std::net::TcpListener;

    #[test]
    fn first_free_port_skips_a_taken_port() {
        // given a port somebody else already holds
        let taken = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        let taken_port = taken.local_addr().expect("addr").port();

        // when
        let next = get_available_port(taken_port, 10).expect("a free port");

        // then it moved on instead of answering with the busy one
        assert_ne!(next, taken_port);
        assert!(next > taken_port);
    }
}
