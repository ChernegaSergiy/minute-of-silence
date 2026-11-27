//! Session-scoped lifecycle for receiving settings over the local network.

use std::{
    sync::{Arc, Mutex as StdMutex},
    time::Instant,
};

use khvylyna_pairing::window::PairingWindow;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;

use super::{mdns, peer_sync};

const RECEIVING_STOPPED_EVENT: &str = "device-receiving-stopped";

struct Session {
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
    pairing: Arc<StdMutex<PairingWindow>>,
}

/// Owns the opt-in receiving session.
#[derive(Default)]
pub struct ReceivingState(Mutex<Option<Session>>);

impl ReceivingState {
    /// Returns whether the receiving session is currently running.
    pub async fn is_enabled(&self) -> bool {
        self.0
            .lock()
            .await
            .as_ref()
            .is_some_and(|session| !session.task.is_finished())
    }

    /// Returns the current code while the receiver can accept a pairing attempt.
    pub async fn pairing_code(&self) -> Option<String> {
        let current = self.0.lock().await;
        let session = current
            .as_ref()
            .filter(|session| !session.task.is_finished())?;
        let pairing = session.pairing.lock().ok()?;
        pairing.code(Instant::now()).map(str::to_owned)
    }

    /// Starts or stops receiving and returns the resulting state.
    pub async fn set_enabled(&self, app: &AppHandle, enabled: bool) -> crate::Result<bool> {
        let mut current = self.0.lock().await;
        if enabled {
            if current
                .as_ref()
                .is_some_and(|session| !session.task.is_finished())
            {
                return Ok(true);
            }
            current.take();
            let pairing = tokio::task::spawn_blocking(|| PairingWindow::new(Instant::now()))
                .await
                .map_err(|error| {
                    crate::AppError::Platform(format!("Pairing setup failed: {error}"))
                })?
                .map_err(|error| {
                    crate::AppError::Platform(format!("Pairing setup failed: {error}"))
                })?;
            let pairing = Arc::new(StdMutex::new(pairing));
            let listener = peer_sync::bind_listener().await?;
            let responder = mdns::prepare_responder()?;
            let version = app.package_info().version.to_string();
            let (shutdown, mut stopped) = watch::channel(false);
            let listener_shutdown = stopped.clone();
            let listener_app = app.clone();
            let listener_pairing = Arc::clone(&pairing);
            let notify_app = app.clone();
            let supervisor_shutdown = shutdown.clone();
            let task = tokio::spawn(async move {
                let mut listener_task = tokio::spawn(peer_sync::run_listener(
                    listener,
                    listener_app,
                    listener_shutdown,
                    listener_pairing,
                ));
                let mut responder_task = tokio::spawn(mdns::run_responder(responder, version));
                tokio::select! {
                    biased;
                    _ = stopped.changed() => {}
                    result = &mut listener_task => {
                        log::warn!("Settings listener stopped unexpectedly: {result:?}");
                    }
                    result = &mut responder_task => {
                        log::warn!("mDNS responder stopped unexpectedly: {result:?}");
                    }
                }
                let requested = *stopped.borrow();
                let _ = supervisor_shutdown.send(true);
                if !listener_task.is_finished() {
                    let _ = listener_task.await;
                }
                if !responder_task.is_finished() {
                    responder_task.abort();
                    let _ = responder_task.await;
                }
                if !requested {
                    if let Err(error) = notify_app.emit(RECEIVING_STOPPED_EVENT, ()) {
                        log::warn!("Failed to report stopped settings receiver: {error}");
                    }
                }
            });
            *current = Some(Session {
                shutdown,
                task,
                pairing,
            });
            Ok(true)
        } else {
            if let Some(session) = current.take() {
                let _ = session.shutdown.send(true);
                let _ = session.task.await;
            }
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn receiving_is_disabled_by_default() {
        let state = ReceivingState::default();
        assert!(!state.is_enabled().await);
        assert!(state.pairing_code().await.is_none());
    }
}
