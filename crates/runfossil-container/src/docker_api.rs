#![doc = "Docker Engine API collector via Unix socket HTTP."]

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

const DOCKER_SOCKET: &str = "/run/docker.sock";
const MAX_RESPONSE_BYTES: u64 = 8_388_608;
const HTTP_TIMEOUT_MS: u64 = 10_000;

pub(crate) fn detect_docker() -> bool {
    Path::new(DOCKER_SOCKET).exists()
}

pub(crate) fn collect_docker_state(store: &mut SnapshotStore) -> Result<(), StoreError> {
    if !detect_docker() {
        return Ok(());
    }
    collect_container_list(store)?;
    Ok(())
}

fn collect_container_list(store: &mut SnapshotStore) -> Result<(), StoreError> {
    let id = "container.docker.containers.json";
    let out_path = "raw/container/docker/containers.json";

    match docker_http_get(DOCKER_SOCKET, "/containers/json?all=true") {
        Ok(body) => {
            let bytes = body.len() as u64;
            store.write_raw_file(Path::new(out_path), &body)?;

            let entry = ManifestEntry::new(
                id,
                SourceSlug::Container,
                "docker",
                "containers",
                ObjectKind::NativeDump,
                ManifestStatus::Captured,
            )
            .with_path(out_path)
            .with_bytes(bytes)
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, HTTP_TIMEOUT_MS, 1, 0));

            store.record_object(entry)?;
        }
        Err(error) => {
            let entry = ManifestEntry::new(
                id,
                SourceSlug::Container,
                "docker",
                "containers",
                ObjectKind::NativeDump,
                ManifestStatus::IoError,
            )
            .with_reason(format!("Docker API containers list failed: {error}"))
            .with_limits(ObjectLimits::new(MAX_RESPONSE_BYTES, HTTP_TIMEOUT_MS, 1, 0));

            store.record_object(entry)?;
        }
    }

    Ok(())
}

fn docker_http_get(socket_path: &str, url_path: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut stream = connect_with_timeout(socket_path)?;

    let request = format!(
        "GET {url_path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        url_path = url_path
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    read_http_response(&mut stream)
}

fn connect_with_timeout(socket_path: &str) -> Result<UnixStream, std::io::Error> {
    let stream = UnixStream::connect(socket_path)?;
    let timeout = Some(Duration::from_millis(HTTP_TIMEOUT_MS));
    stream.set_read_timeout(timeout)?;
    stream.set_write_timeout(timeout)?;
    Ok(stream)
}

fn read_http_response(stream: &mut UnixStream) -> Result<Vec<u8>, std::io::Error> {
    let deadline = Instant::now() + Duration::from_millis(HTTP_TIMEOUT_MS);
    let mut buf = vec![0u8; 65536];
    let mut response = Vec::new();
    let mut headers_end: Option<usize> = None;
    let mut content_length: Option<usize> = None;

    loop {
        if Instant::now() > deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Docker HTTP read timed out",
            ));
        }

        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);

        if headers_end.is_none()
            && let Some(pos) = find_headers_end(&response)
        {
            headers_end = Some(pos);
            content_length = parse_content_length(&response[..pos]);

            if let Some(cl) = content_length {
                let body_start = pos + 4;
                let expected_total = body_start + cl;
                if expected_total > MAX_RESPONSE_BYTES as usize {
                    response.resize(expected_total, 0);
                }
            }
        }

        if let (Some(pos), Some(cl)) = (headers_end, content_length) {
            let body_start = pos + 4;
            if response.len() >= body_start + cl {
                let body = response[body_start..body_start + cl].to_vec();
                validate_http_status(&response, pos)?;
                return Ok(body);
            }
        }
    }

    if let Some(pos) = headers_end {
        let body_start = pos + 4;
        let body = response[body_start..].to_vec();
        validate_http_status(&response, pos)?;
        Ok(body)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "incomplete HTTP response from Docker",
        ))
    }
}

fn find_headers_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length(headers: &[u8]) -> Option<usize> {
    let header_text = std::str::from_utf8(headers).ok()?;
    for line in header_text.lines() {
        if let Some(value) = line
            .to_lowercase()
            .strip_prefix("content-length:")
            .map(|v| v.trim())
        {
            return value.parse().ok();
        }
    }
    None
}

fn validate_http_status(response: &[u8], headers_end: usize) -> Result<(), std::io::Error> {
    let status_line = std::str::from_utf8(&response[..headers_end.min(128)])
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("");

    if status_line.contains("200") {
        return Ok(());
    }

    if status_line.contains("401") || status_line.contains("403") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("Docker API: {status_line}"),
        ));
    }

    Ok(())
}
