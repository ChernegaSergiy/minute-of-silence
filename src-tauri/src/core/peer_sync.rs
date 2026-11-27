//! Local network settings transfer (`POST /api/bundle`) between Minute of Silence peers.
//!
//! Uses lightweight HTTP/1.1 over `tokio::net::TcpStream` / `TcpListener` with zero
//! extra crate dependencies. Incoming bundles are strictly validated with
//! `khvylyna_core::import_bundle_from_json` and forwarded to the frontend as an
//! `incoming-settings-offer` event so the user must explicitly confirm before applying.

use std::{
    io::ErrorKind,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    time,
};

use crate::core::mdns::{DEFAULT_PEER_PORT, local_device_name};

/// HTTP path used to transfer serialized `ExportBundle` JSON payloads.
pub const BUNDLE_ENDPOINT: &str = "/api/bundle";

/// Tauri event emitted to the frontend when another peer offers settings.
pub const INCOMING_SETTINGS_EVENT: &str = "incoming-settings-offer";

/// Timeout for connecting, writing, or reading a peer transfer.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum accepted HTTP request body size (256 KiB).
const MAX_BODY_BYTES: usize = 256 * 1024;

/// Maximum accepted HTTP header section size (8 KiB).
const MAX_HEADER_BYTES: usize = 8 * 1024;

/// Maximum number of concurrent incoming peer connections handled at once.
const MAX_CONCURRENT_CONNECTIONS: usize = 8;

/// Maximum bytes read while waiting for the HTTP response status line.
const MAX_STATUS_LINE_BYTES: usize = 512;

/// Payload emitted to the frontend when a peer sends a valid settings bundle.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingSettingsOffer {
    /// Display name advertised by the sender (or its IP address as fallback).
    pub sender_name: String,
    /// IPv4 address of the sending peer.
    pub sender_ip: String,
    /// Validated `ExportBundle` JSON ready to pass to `import_settings`.
    pub json: String,
}

/// Return `true` if `addr` is a local network destination (RFC 1918 private or link-local).
fn is_allowed_peer_ip(addr: Ipv4Addr) -> bool {
    addr.is_private() || addr.is_link_local() || (cfg!(test) && addr.is_loopback())
}

/// Send a serialized `ExportBundle` JSON payload to a discovered local peer via `POST /api/bundle`.
pub async fn send_bundle(ip: &str, port: u16, json: &str) -> crate::Result<()> {
    let addr: Ipv4Addr = ip
        .trim()
        .parse()
        .map_err(|_| crate::AppError::Platform(format!("Invalid device IP address: {ip}")))?;
    if addr.is_unspecified() || port == 0 || !is_allowed_peer_ip(addr) {
        return Err(crate::AppError::Platform(format!(
            "Refused non-local or invalid device endpoint: {ip}:{port}"
        )));
    }

    let sender = sanitize_header_value(&local_device_name());
    let request = build_http_post(addr, port, &sender, json);
    let target = SocketAddrV4::new(addr, port);

    let mut stream = time::timeout(TRANSFER_TIMEOUT, TcpStream::connect(target))
        .await
        .map_err(|_| crate::AppError::Platform("Connection to device timed out".to_string()))?
        .map_err(|e| crate::AppError::Platform(format!("Failed to connect to device: {e}")))?;

    time::timeout(TRANSFER_TIMEOUT, stream.write_all(&request))
        .await
        .map_err(|_| crate::AppError::Platform("Timed out sending settings".to_string()))?
        .map_err(|e| crate::AppError::Platform(format!("Failed to send settings: {e}")))?;

    let _ = stream.shutdown().await;

    let resp_buf = time::timeout(TRANSFER_TIMEOUT, read_http_status_line(&mut stream))
        .await
        .map_err(|_| {
            crate::AppError::Platform("Timed out waiting for device response".to_string())
        })?
        .map_err(|e| crate::AppError::Platform(format!("Failed to read device response: {e}")))?;

    let status = parse_http_status(&resp_buf).ok_or_else(|| {
        crate::AppError::Platform("Malformed HTTP response from device".to_string())
    })?;

    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(crate::AppError::Platform(format!(
            "Device rejected settings with HTTP {status}"
        )))
    }
}

async fn read_http_status_line(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(128);
    let mut chunk = [0u8; 128];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        let remaining = MAX_STATUS_LINE_BYTES.saturating_sub(buf.len());
        buf.extend_from_slice(&chunk[..n.min(remaining)]);
        if buf.windows(2).any(|w| w == b"\r\n") || buf.len() >= MAX_STATUS_LINE_BYTES {
            break;
        }
    }
    Ok(buf)
}

fn is_retryable_accept_error(err: &std::io::Error) -> bool {
    if matches!(
        err.kind(),
        ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::Interrupted
            | ErrorKind::WouldBlock
            | ErrorKind::TimedOut
    ) {
        return true;
    }

    // Platform-specific file descriptor exhaustion (EMFILE/ENFILE on Unix, WSAEMFILE on Windows)
    #[cfg(unix)]
    if matches!(err.raw_os_error(), Some(23 | 24)) {
        return true;
    }
    #[cfg(windows)]
    if matches!(err.raw_os_error(), Some(10024)) {
        return true;
    }

    false
}

/// Spawn a background HTTP listener on [`DEFAULT_PEER_PORT`] that receives `POST /api/bundle`.
///
/// Valid bundles are never applied silently; they are emitted to the frontend via
/// [`INCOMING_SETTINGS_EVENT`] for explicit user confirmation.
pub fn spawn_listener(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = run_listener(app).await {
            log::info!("Peer settings listener unavailable on port {DEFAULT_PEER_PORT}: {err}");
        }
    });
}

async fn run_listener(app: AppHandle) -> std::io::Result<()> {
    let listener =
        TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DEFAULT_PEER_PORT)).await?;
    let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));
    log::info!("Peer settings listener active on port {DEFAULT_PEER_PORT}");

    loop {
        let (mut stream, peer_addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(err) if is_retryable_accept_error(&err) => {
                log::warn!("Transient peer settings accept error: {err}");
                time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            Err(err) => return Err(err),
        };

        let Ok(permit) = Arc::clone(&permits).try_acquire_owned() else {
            let _ = time::timeout(TRANSFER_TIMEOUT, write_http_status(&mut stream, 503)).await;
            continue;
        };

        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let _permit = permit;
            handle_connection(stream, peer_addr, app_handle).await;
        });
    }
}

async fn handle_connection(mut stream: TcpStream, peer_addr: SocketAddr, app: AppHandle) {
    let parsed = match time::timeout(TRANSFER_TIMEOUT, read_http_request(&mut stream)).await {
        Ok(Ok(req)) => req,
        Ok(Err(status)) => {
            let _ = write_http_status(&mut stream, status).await;
            return;
        }
        Err(_) => {
            let _ = write_http_status(&mut stream, 408).await;
            return;
        }
    };

    if parsed.path != BUNDLE_ENDPOINT {
        let _ = write_http_status(&mut stream, 404).await;
        return;
    }
    if parsed.method != "POST" {
        let _ = write_http_status(&mut stream, 405).await;
        return;
    }

    if let Err(err) = khvylyna_core::import_bundle_from_json(&parsed.body) {
        log::warn!("Rejected invalid incoming settings bundle from {peer_addr}: {err}");
        let _ = write_http_status(&mut stream, 400).await;
        return;
    }

    let sender_ip = peer_addr.ip().to_string();
    let sender_name = parsed
        .sender_name
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| sender_ip.clone());

    let offer = IncomingSettingsOffer {
        sender_name,
        sender_ip,
        json: parsed.body,
    };

    if let Err(err) = app.emit(INCOMING_SETTINGS_EVENT, offer) {
        log::error!("Failed to emit incoming settings offer: {err}");
        let _ = write_http_status(&mut stream, 500).await;
        return;
    }

    let _ = write_http_status(&mut stream, 202).await;
}

struct ParsedHttpRequest {
    method: String,
    path: String,
    sender_name: Option<String>,
    body: String,
}

async fn read_http_request(stream: &mut TcpStream) -> std::result::Result<ParsedHttpRequest, u16> {
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 2048];
    let header_end;

    loop {
        let n = stream.read(&mut chunk).await.map_err(|_| 400u16)?;
        if n == 0 {
            return Err(400);
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&buf) {
            if pos > MAX_HEADER_BYTES {
                return Err(431);
            }
            header_end = pos;
            break;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err(431);
        }
    }

    let header_bytes = &buf[..header_end];
    let header_str = std::str::from_utf8(header_bytes).map_err(|_| 400u16)?;
    let mut lines = header_str.split("\r\n");

    let request_line = lines.next().ok_or(400u16)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(400u16)?.to_string();
    let path = parts.next().ok_or(400u16)?.to_string();

    let mut content_length: usize = 0;
    let mut sender_name: Option<String> = None;

    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let val = v.trim();
            if key.eq_ignore_ascii_case("content-length") {
                content_length = val.parse::<usize>().map_err(|_| 400u16)?;
            } else if key.eq_ignore_ascii_case("x-khvylyna-sender") {
                sender_name = Some(val.to_string());
            }
        }
    }

    if content_length == 0 {
        return Err(400);
    }
    if content_length > MAX_BODY_BYTES {
        return Err(413);
    }

    let body_start = header_end + 4;
    while buf.len().saturating_sub(body_start) < content_length {
        let n = stream.read(&mut chunk).await.map_err(|_| 400u16)?;
        if n == 0 {
            return Err(400);
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > body_start + MAX_BODY_BYTES {
            return Err(413);
        }
    }

    let body_slice = &buf[body_start..body_start + content_length];
    let body = std::str::from_utf8(body_slice)
        .map_err(|_| 400u16)?
        .to_string();

    Ok(ParsedHttpRequest {
        method,
        path,
        sender_name,
        body,
    })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

async fn write_http_status(stream: &mut TcpStream, code: u16) -> std::io::Result<()> {
    let reason = match code {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    let response =
        format!("HTTP/1.1 {code} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}

fn sanitize_header_value(raw: &str) -> String {
    raw.chars().filter(|ch| !ch.is_control()).take(64).collect()
}

/// Build an HTTP/1.1 `POST /api/bundle` request carrying a JSON payload.
pub fn build_http_post(ip: Ipv4Addr, port: u16, sender: &str, json: &str) -> Vec<u8> {
    let mut req = format!(
        "POST {BUNDLE_ENDPOINT} HTTP/1.1\r\n\
         Host: {ip}:{port}\r\n\
         Content-Type: application/json\r\n\
         X-Khvylyna-Sender: {sender}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        json.len()
    )
    .into_bytes();
    req.extend_from_slice(json.as_bytes());
    req
}

/// Parse the numeric HTTP status code from the first line of an HTTP response.
pub fn parse_http_status(resp: &[u8]) -> Option<u16> {
    let line_end = resp
        .windows(2)
        .position(|w| w == b"\r\n")
        .unwrap_or(resp.len());
    let first_line = std::str::from_utf8(&resp[..line_end]).ok()?;
    let mut parts = first_line.split_whitespace();
    let version = parts.next()?;
    if !version.starts_with("HTTP/") {
        return None;
    }
    parts.next()?.parse::<u16>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_http_post_formats_valid_request() {
        let req = build_http_post(
            Ipv4Addr::new(192, 168, 1, 105),
            80,
            "Office Laptop",
            r#"{"ok":true}"#,
        );
        let text = std::str::from_utf8(&req).expect("utf-8 request");
        assert!(text.starts_with("POST /api/bundle HTTP/1.1\r\n"));
        assert!(text.contains("Host: 192.168.1.105:80\r\n"));
        assert!(text.contains("X-Khvylyna-Sender: Office Laptop\r\n"));
        assert!(text.contains("Content-Length: 11\r\n"));
        assert!(text.ends_with("\r\n\r\n{\"ok\":true}"));
    }

    #[test]
    fn parse_http_status_extracts_status_codes() {
        assert_eq!(parse_http_status(b"HTTP/1.1 200 OK\r\n\r\n"), Some(200));
        assert_eq!(
            parse_http_status(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n"),
            Some(202)
        );
        assert_eq!(
            parse_http_status(b"HTTP/1.0 400 Bad Request\r\n"),
            Some(400)
        );
        assert_eq!(parse_http_status(b"NOT_HTTP 200 OK"), None);
    }

    #[tokio::test]
    async fn send_bundle_round_trips_over_loopback() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();

        let bundle = khvylyna_core::ExportBundle {
            settings: khvylyna_core::Settings::default(),
            personal_dates: Vec::new(),
        };
        let payload = khvylyna_core::export_bundle_to_json(&bundle).expect("serialize bundle");

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let parsed = read_http_request(&mut stream).await.expect("parse req");
            assert_eq!(parsed.method, "POST");
            assert_eq!(parsed.path, BUNDLE_ENDPOINT);
            let imported = khvylyna_core::import_bundle_from_json(&parsed.body)
                .expect("valid bundle received");
            assert_eq!(imported.settings.version, bundle.settings.version);
            write_http_status(&mut stream, 202)
                .await
                .expect("write 202");
        });

        send_bundle("127.0.0.1", port, &payload)
            .await
            .expect("send_bundle succeeds");
        server.await.expect("server task finishes cleanly");
    }

    #[tokio::test]
    async fn send_bundle_rejects_public_and_invalid_ips() {
        assert!(send_bundle("8.8.8.8", 80, "{}").await.is_err());
        assert!(send_bundle("1.1.1.1", 49152, "{}").await.is_err());
        assert!(send_bundle("0.0.0.0", 80, "{}").await.is_err());
        assert!(send_bundle("192.168.1.10", 0, "{}").await.is_err());
        assert!(is_allowed_peer_ip(Ipv4Addr::new(192, 168, 1, 105)));
        assert!(is_allowed_peer_ip(Ipv4Addr::new(10, 0, 0, 5)));
        assert!(is_allowed_peer_ip(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_allowed_peer_ip(Ipv4Addr::new(169, 254, 1, 2)));
        assert!(!is_allowed_peer_ip(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn sanitize_header_value_keeps_cyrillic_and_strips_controls() {
        assert_eq!(sanitize_header_value("Ноутбук Сергія"), "Ноутбук Сергія");
        assert_eq!(
            sanitize_header_value("Evil\r\nX-Injected: true\0Name"),
            "EvilX-Injected: trueName"
        );
        let long_name = "А".repeat(100);
        assert_eq!(sanitize_header_value(&long_name).chars().count(), 64);
    }
}
