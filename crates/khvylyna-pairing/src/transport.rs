//! Authenticated, encrypted transfer over an established TCP connection.

use std::{
    io,
    sync::{Arc, Mutex},
    time::Instant,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use zeroize::Zeroizing;

use crate::{
    SenderExchange,
    channel::{self, Role, SealingKey},
    window::PairingWindow,
};

const MAGIC: &[u8; 8] = b"KHVSYNC1";
const MAX_HANDSHAKE_BYTES: usize = 1024;
const MAX_ENCRYPTED_BYTES: usize = 256 * 1024 + 28;
const ACCEPTED: &[u8] = b"accepted";
const REJECTED: &[u8] = b"rejected";

fn invalid_data() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Protected transfer failed")
}

async fn write_frame(stream: &mut TcpStream, data: &[u8], limit: usize) -> io::Result<()> {
    if data.len() > limit {
        return Err(invalid_data());
    }
    let length = u32::try_from(data.len()).map_err(|_| invalid_data())?;
    stream.write_u32(length).await?;
    stream.write_all(data).await
}

async fn read_frame(stream: &mut TcpStream, limit: usize) -> io::Result<Vec<u8>> {
    let length = stream.read_u32().await? as usize;
    if length > limit {
        return Err(invalid_data());
    }
    let mut data = vec![0; length];
    stream.read_exact(&mut data).await?;
    Ok(data)
}

/// Authenticates the receiver before encrypting and sending a single payload.
pub async fn send(stream: &mut TcpStream, code: &str, payload: &[u8]) -> io::Result<()> {
    let (sender, request) = SenderExchange::start(code).map_err(|_| invalid_data())?;
    stream.write_all(MAGIC).await?;
    write_frame(stream, &request, MAX_HANDSHAKE_BYTES).await?;
    let response = read_frame(stream, MAX_HANDSHAKE_BYTES).await?;
    let code = Zeroizing::new(code.to_owned());
    let (confirmation, session_key) =
        tokio::task::spawn_blocking(move || sender.finish(&code, &response))
            .await
            .map_err(|_| invalid_data())?
            .map_err(|_| invalid_data())?;
    let (seal, open) =
        channel::derive_keys(&session_key, Role::Sender).map_err(|_| invalid_data())?;
    let ciphertext = seal.seal(payload).map_err(|_| invalid_data())?;
    write_frame(stream, &confirmation, MAX_HANDSHAKE_BYTES).await?;
    write_frame(stream, &ciphertext, MAX_ENCRYPTED_BYTES).await?;
    let reply = read_frame(stream, MAX_HANDSHAKE_BYTES).await?;
    let status = open.open(&reply).map_err(|_| invalid_data())?;
    if status.as_slice() == ACCEPTED {
        Ok(())
    } else {
        Err(invalid_data())
    }
}

/// Authenticates the sender and decrypts one payload.
pub async fn receive(
    stream: &mut TcpStream,
    window: Arc<Mutex<PairingWindow>>,
) -> io::Result<(Zeroizing<Vec<u8>>, SealingKey)> {
    let mut magic = [0; MAGIC.len()];
    stream.read_exact(&mut magic).await?;
    if &magic != MAGIC {
        return Err(invalid_data());
    }
    let request = read_frame(stream, MAX_HANDSHAKE_BYTES).await?;
    let start_window = Arc::clone(&window);
    let (receiver, response) = tokio::task::spawn_blocking(move || {
        let mut guard = start_window.lock().map_err(|_| invalid_data())?;
        guard
            .start(&request, Instant::now())
            .map_err(|_| invalid_data())
    })
    .await
    .map_err(|_| invalid_data())??;
    write_frame(stream, &response, MAX_HANDSHAKE_BYTES).await?;
    let confirmation = read_frame(stream, MAX_HANDSHAKE_BYTES).await?;
    let key = tokio::task::spawn_blocking(move || {
        let mut guard = window.lock().map_err(|_| invalid_data())?;
        guard
            .finish(receiver, &confirmation, Instant::now())
            .map_err(|_| invalid_data())
    })
    .await
    .map_err(|_| invalid_data())??;
    let (seal, open) = channel::derive_keys(&key, Role::Receiver).map_err(|_| invalid_data())?;
    let ciphertext = read_frame(stream, MAX_ENCRYPTED_BYTES).await?;
    let payload = open.open(&ciphertext).map_err(|_| invalid_data())?;
    Ok((payload, seal))
}

/// Sends one authenticated result after the receiving application handles the payload.
pub async fn respond(stream: &mut TcpStream, key: SealingKey, accepted: bool) -> io::Result<()> {
    let status = if accepted { ACCEPTED } else { REJECTED };
    let ciphertext = key.seal(status).map_err(|_| invalid_data())?;
    write_frame(stream, &ciphertext, MAX_HANDSHAKE_BYTES).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn authenticates_before_transferring_and_acknowledging() {
        let window = Arc::new(Mutex::new(
            PairingWindow::new(Instant::now()).expect("window"),
        ));
        let code = window
            .lock()
            .expect("lock")
            .code(Instant::now())
            .expect("code")
            .to_string();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let (payload, response) = receive(&mut stream, window).await.expect("receive");
            assert_eq!(payload.as_slice(), b"private settings");
            respond(&mut stream, response, true).await.expect("respond");
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
    async fn wrong_code_never_delivers_a_payload() {
        let window = Arc::new(Mutex::new(
            PairingWindow::new(Instant::now()).expect("window"),
        ));
        let actual_code = window
            .lock()
            .expect("lock")
            .code(Instant::now())
            .expect("code")
            .to_string();
        let wrong_code = if actual_code == "000000" {
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
        assert!(
            send(&mut stream, wrong_code, b"private settings")
                .await
                .is_err()
        );
        drop(stream);
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("server timeout")
            .expect("server");
    }

    #[tokio::test]
    async fn legacy_plaintext_is_not_accepted() {
        let window = Arc::new(Mutex::new(
            PairingWindow::new(Instant::now()).expect("window"),
        ));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            assert!(receive(&mut stream, window).await.is_err());
        });
        let mut stream = TcpStream::connect(address).await.expect("connect");
        stream
            .write_all(b"POST /api/bundle HTTP/1.1\r\n")
            .await
            .expect("write");
        server.await.expect("server");
    }
}
