//! Authenticated, encrypted settings transfer over Tokio TCP streams.

use alloc::{string::ToString, vec, vec::Vec};
use std::io;

use rand::rngs::OsRng;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use zeroize::Zeroizing;

use crate::{
    ProtocolError,
    channel::SessionSecret,
    now_ms,
    pake::SenderExchange,
    protocol::{self, DESKTOP_MAX_PAYLOAD, FrameHeader, FrameKind},
    window::PairingWindow,
};

/// Maximum settings payload accepted by desktop and Android receivers.
pub const MAX_PAYLOAD: u16 = DESKTOP_MAX_PAYLOAD;

fn io_error(error: impl core::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

fn error_code(error: &ProtocolError) -> u8 {
    match error {
        ProtocolError::UnsupportedVersion => protocol::ERROR_UNSUPPORTED_VERSION,
        ProtocolError::UnsupportedSuite => protocol::ERROR_UNSUPPORTED_SUITE,
        ProtocolError::WindowUnavailable => protocol::ERROR_WINDOW_UNAVAILABLE,
        ProtocolError::PayloadTooLarge => protocol::ERROR_PAYLOAD_TOO_LARGE,
        _ => protocol::ERROR_INVALID_MESSAGE,
    }
}

async fn write_frame(stream: &mut TcpStream, kind: FrameKind, payload: &[u8]) -> io::Result<()> {
    let mut header = [0u8; protocol::HEADER_LEN];
    protocol::write_header(kind, payload.len(), &mut header).map_err(io_error)?;
    stream.write_all(&header).await?;
    stream.write_all(payload).await
}

async fn read_frame(
    stream: &mut TcpStream,
    negotiated_max: u16,
) -> Result<(FrameHeader, Vec<u8>), ProtocolError> {
    let mut header = [0u8; protocol::HEADER_LEN];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|_| ProtocolError::InvalidFrame)?;
    let parsed = protocol::parse_header(&header, negotiated_max)?;
    let mut payload = vec![0u8; parsed.len];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(|_| ProtocolError::InvalidFrame)?;
    Ok((parsed, payload))
}

async fn read_expected(
    stream: &mut TcpStream,
    kind: FrameKind,
    negotiated_max: u16,
) -> Result<Vec<u8>, ProtocolError> {
    let (header, payload) = read_frame(stream, negotiated_max).await?;
    if header.kind != kind {
        return Err(ProtocolError::InvalidFrame);
    }
    Ok(payload)
}

fn negotiated_limit(offer: &[u8], local_max: u16) -> Result<u16, ProtocolError> {
    let bytes: [u8; 2] = offer.try_into().map_err(|_| ProtocolError::InvalidFrame)?;
    let offered = u16::from_be_bytes(bytes);
    if offered == 0 {
        return Err(ProtocolError::PayloadTooLarge);
    }
    Ok(offered.min(local_max))
}

/// Authenticates the receiver before encrypting and sending one payload.
pub async fn send(stream: &mut TcpStream, code: &str, payload: &[u8]) -> io::Result<()> {
    if payload.len() > MAX_PAYLOAD as usize {
        return Err(io_error(ProtocolError::PayloadTooLarge));
    }
    write_frame(stream, FrameKind::Hello, &MAX_PAYLOAD.to_be_bytes()).await?;
    let answer = read_frame(stream, 0).await.map_err(io_error)?;
    if answer.0.kind != FrameKind::HelloAck {
        return Err(io_error(ProtocolError::InvalidFrame));
    }
    let limit = negotiated_limit(&answer.1, MAX_PAYLOAD).map_err(io_error)?;

    let (sender, request) = SenderExchange::start(code, limit, &mut OsRng).map_err(io_error)?;
    write_frame(stream, FrameKind::PakeInit, &request).await?;
    let response = read_expected(stream, FrameKind::PakeResponse, limit)
        .await
        .map_err(io_error)?;
    let secret = sender.finish(code, &response).map_err(io_error)?;
    let confirm = secret.seal_confirm(&mut OsRng).map_err(io_error)?;
    write_frame(stream, FrameKind::Confirm, &confirm).await?;

    let ciphertext = secret.seal_data(payload, &mut OsRng).map_err(io_error)?;
    write_frame(stream, FrameKind::Data, &ciphertext).await?;

    let ack = read_expected(stream, FrameKind::Ack, limit)
        .await
        .map_err(io_error)?;
    if !secret.open_ack(&ack).map_err(io_error)? {
        return Err(io_error(ProtocolError::TransferRejected));
    }
    Ok(())
}

/// Authenticates the sender and decrypts one payload.
pub async fn receive(
    stream: &mut TcpStream,
    window: std::sync::Arc<std::sync::Mutex<PairingWindow>>,
) -> io::Result<(Zeroizing<Vec<u8>>, SessionSecret)> {
    let hello = match read_frame(stream, 0).await {
        Ok((header, payload)) if header.kind == FrameKind::Hello => payload,
        Ok(_) => return Err(io_error(ProtocolError::InvalidFrame)),
        Err(error) => {
            let code = [error_code(&error)];
            let _ = write_frame(stream, FrameKind::ErrorFrame, &code).await;
            return Err(io_error(error));
        }
    };
    let limit = negotiated_limit(&hello, MAX_PAYLOAD).map_err(io_error)?;
    write_frame(stream, FrameKind::HelloAck, &limit.to_be_bytes()).await?;

    let request = read_expected(stream, FrameKind::PakeInit, limit)
        .await
        .map_err(io_error)?;
    let (attempt, response) = {
        let mut guard = window.lock().map_err(io_error)?;
        guard
            .start(&request, now_ms(), limit, &mut OsRng)
            .map_err(io_error)?
    };
    write_frame(stream, FrameKind::PakeResponse, &response).await?;

    let confirm = read_expected(stream, FrameKind::Confirm, limit)
        .await
        .map_err(io_error)?;
    attempt.verify_confirm(&confirm).map_err(io_error)?;
    let secret = {
        let mut guard = window.lock().map_err(io_error)?;
        guard.finish(attempt, now_ms()).map_err(io_error)?
    };

    let ciphertext = read_expected(stream, FrameKind::Data, limit)
        .await
        .map_err(io_error)?;
    let payload = secret.open_data(&ciphertext).map_err(io_error)?;
    Ok((payload, secret))
}

/// Sends one authenticated result after the receiving application handles the payload.
pub async fn respond(
    stream: &mut TcpStream,
    secret: SessionSecret,
    accepted: bool,
) -> io::Result<()> {
    let ciphertext = secret.seal_ack(accepted, &mut OsRng).map_err(io_error)?;
    write_frame(stream, FrameKind::Ack, &ciphertext).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::ToOwned;
    use std::{sync::Arc, sync::Mutex, time::Duration};
    use tokio::net::TcpListener;

    fn test_window() -> Arc<Mutex<PairingWindow>> {
        Arc::new(Mutex::new(
            PairingWindow::new(now_ms(), &mut OsRng).expect("window"),
        ))
    }

    #[tokio::test]
    async fn authenticates_before_transferring_and_acknowledging() {
        let window = test_window();
        let code = window
            .lock()
            .expect("lock")
            .code(now_ms())
            .expect("code")
            .to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let (payload, secret) = receive(&mut stream, window).await.expect("receive");
            assert_eq!(payload.as_slice(), b"private settings");
            respond(&mut stream, secret, true).await.expect("respond");
        });
        let mut stream = TcpStream::connect(address).await.expect("connect");
        tokio::time::timeout(
            Duration::from_secs(10),
            send(&mut stream, &code, b"private settings"),
        )
        .await
        .expect("transfer timeout")
        .expect("send");
        server.await.expect("server");
    }

    #[tokio::test]
    async fn rejected_ack_is_reported_distinctly() {
        let window = test_window();
        let code = window
            .lock()
            .expect("lock")
            .code(now_ms())
            .expect("code")
            .to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let (payload, secret) = receive(&mut stream, window).await.expect("receive");
            assert_eq!(payload.as_slice(), b"private settings");
            respond(&mut stream, secret, false).await.expect("respond");
        });
        let mut stream = TcpStream::connect(address).await.expect("connect");
        let error = send(&mut stream, &code, b"private settings")
            .await
            .expect_err("rejected transfer");
        assert_eq!(
            error.to_string(),
            ProtocolError::TransferRejected.to_string()
        );
        server.await.expect("server");
    }

    #[tokio::test]
    async fn wrong_code_never_delivers_a_payload() {
        let window = test_window();
        let actual = window
            .lock()
            .expect("lock")
            .code(now_ms())
            .expect("code")
            .to_owned();
        let wrong = if actual == "000000" {
            "000001"
        } else {
            "000000"
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            assert!(receive(&mut stream, window).await.is_err());
        });
        let mut stream = TcpStream::connect(address).await.expect("connect");
        assert!(send(&mut stream, wrong, b"private settings").await.is_err());
        drop(stream);
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("server timeout")
            .expect("server");
    }

    #[tokio::test]
    async fn oversized_payload_is_rejected_before_any_frame_is_sent() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut byte = [0u8; 1];
            let read = tokio::time::timeout(Duration::from_millis(250), stream.read(&mut byte))
                .await
                .expect("read timeout")
                .expect("read");
            assert_eq!(read, 0);
        });
        let mut stream = TcpStream::connect(address).await.expect("connect");
        let payload = vec![0u8; MAX_PAYLOAD as usize + 1];
        let error = send(&mut stream, "123456", &payload)
            .await
            .expect_err("oversized payload");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        drop(stream);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn legacy_plaintext_and_unknown_versions_are_rejected() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            assert!(receive(&mut stream, test_window()).await.is_err());
            let (mut stream, _) = listener.accept().await.expect("second accept");
            assert!(receive(&mut stream, test_window()).await.is_err());
        });
        let mut stream = TcpStream::connect(address).await.expect("connect");
        stream
            .write_all(b"POST /api/bundle HTTP/1.1\r\n")
            .await
            .expect("legacy write");
        drop(stream);

        let mut stream = TcpStream::connect(address).await.expect("connect");
        let mut header = [0u8; protocol::HEADER_LEN];
        protocol::write_header(FrameKind::Hello, 2, &mut header).expect("header");
        header[protocol::MAGIC.len()] = protocol::VERSION + 1;
        stream.write_all(&header).await.expect("version write");
        stream
            .write_all(&2u16.to_be_bytes())
            .await
            .expect("payload");
        drop(stream);

        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("server timeout")
            .expect("server");
    }
}
