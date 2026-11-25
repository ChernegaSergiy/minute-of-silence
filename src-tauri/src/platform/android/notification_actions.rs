//! Receives native notification actions without involving the webview.

use super::{bridge, schedule};
use crate::app::notification_actions::NotificationActionRequest;
use crate::{AppState, Result};
use serde::Serialize;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize)]
struct RegisterActionHandler {
    handler: Channel,
}

/// Registers the native action channel and processes requests in arrival order.
pub async fn register(app: AppHandle) -> Result<()> {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let handler = Channel::new(move |body| {
        if let InvokeResponseBody::Json(payload) = body {
            match serde_json::from_str::<NotificationActionRequest>(&payload) {
                Ok(request) => {
                    if sender.send(request).is_err() {
                        log::warn!("The notification action handler is no longer running");
                    }
                }
                Err(error) => log::warn!("Invalid notification action: {error}"),
            }
        }
        Ok(())
    });
    bridge::call_async::<()>(
        "registerNotificationActionHandler",
        RegisterActionHandler { handler },
    )
    .await?;

    tauri::async_runtime::spawn(async move {
        while let Some(request) = receiver.recv().await {
            let app = app.clone();
            match tauri::async_runtime::spawn_blocking(move || apply_request(&app, request)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => log::warn!("Failed to apply notification action: {error}"),
                Err(error) => log::warn!("Notification action task failed: {error}"),
            }
        }
    });
    Ok(())
}

fn apply_request(app: &AppHandle, request: NotificationActionRequest) -> Result<()> {
    let state = app.state::<AppState>();
    let mut changed = false;
    state.update_settings(|settings| {
        changed = request.apply(settings, chrono::Local::now());
    })?;
    if changed {
        let _ = app.emit("status-updated", ());
    } else {
        let (settings, generation) = state.schedule_configuration();
        schedule::configure(&settings, generation)?;
    }
    Ok(())
}
