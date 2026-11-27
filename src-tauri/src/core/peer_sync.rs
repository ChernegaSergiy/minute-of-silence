//! Authenticated local network settings transfer between Minute of Silence peers.

use std::{
    collections::VecDeque,
    future::Future,
    io::ErrorKind,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, Mutex},
    time::Duration,
};

use khvylyna_pairing::{transport, window::PairingWindow};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Semaphore, watch},
    task::JoinSet,
    time,
};

use crate::core::mdns::{DEFAULT_PEER_PORT, local_device_name};

/// Tauri event emitted to the frontend when another peer offers settings.
pub const INCOMING_SETTINGS_EVENT: &str = "incoming-settings-offer";

/// Timeout for connecting, writing, or reading a peer transfer.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(20);

/// Maximum number of concurrent incoming peer connections handled at once.
const MAX_CONCURRENT_CONNECTIONS: usize = 8;

const MAX_PENDING_OFFERS: usize = 8;

/// Event emitted when a pairing attempt may have changed the code's availability.
pub const PAIRING_CODE_CHANGED_EVENT: &str = "pairing-code-changed";

#[derive(Serialize, Deserialize)]
struct TransferPayload {
    sender_name: String,
    json: String,
}

/// Payload emitted to the frontend when a peer sends a valid settings bundle.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingSettingsOffer {
    /// Identifier used to acknowledge the displayed offer.
    pub id: u64,
    /// Display name advertised by the sender (or its IP address as fallback).
    pub sender_name: String,
    /// IPv4 address of the sending peer.
    pub sender_ip: String,
    /// Validated `ExportBundle` JSON ready to pass to `import_settings`.
    pub json: String,
}

#[derive(Default)]
struct PendingOffers {
    next_id: u64,
    offers: VecDeque<IncomingSettingsOffer>,
}

/// Incoming settings awaiting an explicit user decision.
#[derive(Default)]
pub struct IncomingOfferQueue(Mutex<PendingOffers>);

impl IncomingOfferQueue {
    fn enqueue(&self, mut offer: IncomingSettingsOffer) -> bool {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if pending.offers.len() >= MAX_PENDING_OFFERS {
            return false;
        }
        pending.next_id += 1;
        offer.id = pending.next_id;
        pending.offers.push_back(offer);
        true
    }

    /// Returns the oldest offer without removing it.
    pub fn next(&self) -> Option<IncomingSettingsOffer> {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .offers
            .front()
            .cloned()
    }

    /// Removes the oldest offer only if it matches the supplied identifier.
    pub fn dismiss(&self, id: u64) {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if pending.offers.front().is_some_and(|offer| offer.id == id) {
            pending.offers.pop_front();
        }
    }
}

/// Return `true` if `addr` is a local network destination (RFC 1918 private or link-local).
fn is_allowed_peer_ip(addr: Ipv4Addr) -> bool {
    addr.is_private() || addr.is_link_local() || (cfg!(test) && addr.is_loopback())
}

/// Sends settings only after the receiver authenticates with the pairing code.
pub async fn send_bundle(ip: &str, port: u16, json: &str, code: &str) -> crate::Result<()> {
    let addr: Ipv4Addr = ip
        .trim()
        .parse()
        .map_err(|_| crate::AppError::Platform(format!("Invalid device IP address: {ip}")))?;
    if addr.is_unspecified() || port == 0 || !is_allowed_peer_ip(addr) {
        return Err(crate::AppError::Platform(format!(
            "Refused non-local or invalid device endpoint: {ip}:{port}"
        )));
    }
    let payload = serde_json::to_vec(&TransferPayload {
        sender_name: sanitize_sender_name(&local_device_name()),
        json: json.to_string(),
    })?;
    let transfer = async {
        let mut stream = TcpStream::connect(SocketAddrV4::new(addr, port)).await?;
        transport::send(&mut stream, code, &payload).await
    };
    time::timeout(TRANSFER_TIMEOUT, transfer)
        .await
        .map_err(|_| crate::AppError::Platform("Secure transfer timed out".to_string()))?
        .map_err(|error| crate::AppError::Platform(format!("Secure transfer failed: {error}")))
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

pub(crate) async fn bind_listener() -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DEFAULT_PEER_PORT)).await
}

pub(crate) async fn run_listener(
    listener: TcpListener,
    app: AppHandle,
    shutdown: watch::Receiver<bool>,
    pairing: Arc<Mutex<PairingWindow>>,
) -> std::io::Result<()> {
    run_accept_loop(listener, shutdown, move |stream, peer_addr| {
        handle_connection(stream, peer_addr, app.clone(), Arc::clone(&pairing))
    })
    .await
}

async fn run_accept_loop<F, Connection>(
    listener: TcpListener,
    mut shutdown: watch::Receiver<bool>,
    handle: F,
) -> std::io::Result<()>
where
    F: Fn(TcpStream, SocketAddr) -> Connection,
    Connection: Future<Output = ()> + Send + 'static,
{
    let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));
    let mut connections = JoinSet::new();
    log::info!("Peer settings listener active on port {DEFAULT_PEER_PORT}");

    loop {
        let accepted = tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            Some(_) = connections.join_next(), if !connections.is_empty() => continue,
            accepted = listener.accept() => accepted,
        };
        let (stream, peer_addr) = match accepted {
            Ok(pair) => pair,
            Err(err) if is_retryable_accept_error(&err) => {
                log::warn!("Transient peer settings accept error: {err}");
                time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            Err(err) => return Err(err),
        };

        let Ok(permit) = Arc::clone(&permits).try_acquire_owned() else {
            continue;
        };

        let connection = handle(stream, peer_addr);
        connections.spawn(async move {
            let _permit = permit;
            connection.await;
        });
    }
    connections.shutdown().await;
    Ok(())
}

async fn handle_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    app: AppHandle,
    pairing: Arc<Mutex<PairingWindow>>,
) {
    let received = time::timeout(TRANSFER_TIMEOUT, transport::receive(&mut stream, pairing)).await;
    if let Err(error) = app.emit(PAIRING_CODE_CHANGED_EVENT, ()) {
        log::warn!("Failed to report pairing status: {error}");
    }
    let (payload, secret) = match received {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            log::debug!("Rejected unauthenticated settings transfer from {peer_addr}: {error}");
            return;
        }
        Err(_) => {
            log::debug!("Settings transfer timed out from {peer_addr}");
            return;
        }
    };

    let accepted = match serde_json::from_slice::<TransferPayload>(&payload) {
        Ok(transfer) => {
            if let Err(error) = khvylyna_core::import_bundle_from_json(&transfer.json) {
                log::warn!("Rejected invalid incoming settings bundle from {peer_addr}: {error}");
                false
            } else {
                let sender_ip = peer_addr.ip().to_string();
                let sender_name = sanitize_sender_name(&transfer.sender_name);
                let offer = IncomingSettingsOffer {
                    id: 0,
                    sender_name: if sender_name.is_empty() {
                        sender_ip.clone()
                    } else {
                        sender_name
                    },
                    sender_ip,
                    json: transfer.json,
                };
                app.state::<IncomingOfferQueue>().enqueue(offer)
            }
        }
        Err(_) => false,
    };

    if accepted {
        if let Err(error) = app.emit(INCOMING_SETTINGS_EVENT, ()) {
            log::error!("Failed to emit incoming settings offer: {error}");
        }
    }

    match time::timeout(
        TRANSFER_TIMEOUT,
        transport::respond(&mut stream, secret, accepted),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log::debug!("Settings transfer response failed: {error}"),
        Err(error) => log::debug!("Settings transfer response timed out: {error}"),
    }
}

fn sanitize_sender_name(raw: &str) -> String {
    raw.chars().filter(|ch| !ch.is_control()).take(64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use khvylyna_pairing::now_ms;
    use rand::rngs::OsRng;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn stopping_listener_closes_connections_and_releases_port() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind loopback");
        let address = listener.local_addr().expect("listener address");
        let (shutdown, receiver) = watch::channel(false);
        let (accepted, mut received) = tokio::sync::mpsc::channel(1);
        let server = tokio::spawn(run_accept_loop(listener, receiver, move |stream, _| {
            let accepted = accepted.clone();
            async move {
                let _stream = stream;
                let _ = accepted.send(()).await;
                std::future::pending::<()>().await;
            }
        }));
        let mut client = TcpStream::connect(address).await.expect("connect");
        time::timeout(Duration::from_secs(2), received.recv())
            .await
            .expect("accepted in time")
            .expect("connection accepted");
        shutdown.send(true).expect("request shutdown");
        time::timeout(Duration::from_secs(2), server)
            .await
            .expect("listener stopped in time")
            .expect("listener task")
            .expect("listener result");
        let mut buffer = [0u8; 1];
        assert_eq!(client.read(&mut buffer).await.expect("read EOF"), 0);
        let rebound = TcpListener::bind(address).await.expect("port released");
        drop(rebound);
    }

    fn offer(sender: &str) -> IncomingSettingsOffer {
        IncomingSettingsOffer {
            id: 0,
            sender_name: sender.to_string(),
            sender_ip: "192.168.1.10".to_string(),
            json: "{}".to_string(),
        }
    }

    #[test]
    fn incoming_offers_remain_until_acknowledged_in_order() {
        let queue = IncomingOfferQueue::default();
        assert!(queue.enqueue(offer("first")));
        assert!(queue.enqueue(offer("second")));
        let first = queue.next().expect("first offer");
        assert_eq!(first.sender_name, "first");
        assert_eq!(queue.next().expect("offer remains pending").id, first.id);
        queue.dismiss(first.id + 1);
        assert_eq!(queue.next().expect("first offer remains").id, first.id);
        queue.dismiss(first.id);
        let second = queue.next().expect("second offer");
        assert_eq!(second.sender_name, "second");
        assert_ne!(first.id, second.id);
        queue.dismiss(second.id);
        assert!(queue.next().is_none());
    }

    #[test]
    fn incoming_offer_queue_rejects_new_offers_when_full() {
        let queue = IncomingOfferQueue::default();
        for _ in 0..MAX_PENDING_OFFERS {
            assert!(queue.enqueue(offer("sender")));
        }
        assert!(!queue.enqueue(offer("overflow")));
        let first = queue.next().expect("first offer");
        queue.dismiss(first.id);
        assert!(queue.enqueue(offer("new sender")));
    }

    #[tokio::test]
    async fn send_bundle_round_trips_over_authenticated_loopback() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();
        let window = Arc::new(Mutex::new(
            PairingWindow::new(now_ms(), &mut OsRng).expect("pairing window"),
        ));
        let code = window
            .lock()
            .expect("pairing lock")
            .code(now_ms())
            .expect("pairing code")
            .to_string();
        let bundle = khvylyna_core::ExportBundle {
            settings: khvylyna_core::Settings::default(),
            personal_dates: Vec::new(),
        };
        let payload = khvylyna_core::export_bundle_to_json(&bundle).expect("serialize bundle");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let (payload, response) = transport::receive(&mut stream, window)
                .await
                .expect("receive");
            let transfer: TransferPayload = serde_json::from_slice(&payload).expect("transfer");
            let imported = khvylyna_core::import_bundle_from_json(&transfer.json)
                .expect("valid bundle received");
            assert_eq!(imported.settings.version, bundle.settings.version);
            transport::respond(&mut stream, response, true)
                .await
                .expect("respond");
        });
        send_bundle("127.0.0.1", port, &payload, &code)
            .await
            .expect("secure transfer succeeds");
        server.await.expect("server");
    }

    #[tokio::test]
    async fn send_bundle_rejects_public_and_invalid_ips() {
        assert!(send_bundle("8.8.8.8", 80, "{}", "123456").await.is_err());
        assert!(send_bundle("1.1.1.1", 49152, "{}", "123456").await.is_err());
        assert!(send_bundle("0.0.0.0", 80, "{}", "123456").await.is_err());
        assert!(
            send_bundle("192.168.1.10", 0, "{}", "123456")
                .await
                .is_err()
        );
        assert!(is_allowed_peer_ip(Ipv4Addr::new(192, 168, 1, 105)));
        assert!(is_allowed_peer_ip(Ipv4Addr::new(10, 0, 0, 5)));
        assert!(is_allowed_peer_ip(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_allowed_peer_ip(Ipv4Addr::new(169, 254, 1, 2)));
        assert!(!is_allowed_peer_ip(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn sanitize_sender_name_keeps_cyrillic_and_strips_controls() {
        assert_eq!(sanitize_sender_name("Ноутбук Сергія"), "Ноутбук Сергія");
        assert_eq!(
            sanitize_sender_name("Evil\r\nX-Injected: true\0Name"),
            "EvilX-Injected: trueName"
        );
        let long_name = "А".repeat(100);
        assert_eq!(sanitize_sender_name(&long_name).chars().count(), 64);
    }
}
