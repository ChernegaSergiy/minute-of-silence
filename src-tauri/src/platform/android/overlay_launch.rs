//! Coordinates full-screen ceremony launches with the Android activity.

use crate::error::Result;
use crate::state::{AppState, CeremonyKind, CeremonyPhase};
use serde_json::json;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

static OVERLAY_COMMANDS: Mutex<()> = Mutex::const_new(());

async fn update_active_overlay(app: &AppHandle, ceremony_id: u64, command: &str) -> Result<()> {
    let _guard = OVERLAY_COMMANDS.lock().await;
    let state = app.state::<AppState>();
    let should_update = {
        let inner = state.lock();
        inner.settings.show_visual_overlay
            && inner.active_ceremony.is_some_and(|active| {
                active.id == ceremony_id
                    && active.kind == CeremonyKind::Scheduled
                    && active.phase != CeremonyPhase::Finishing
            })
    };
    if should_update {
        super::bridge::call_async::<()>(command, json!({ "ceremonyId": ceremony_id.to_string() }))
            .await?;
    }
    Ok(())
}

/// Shows the overlay over the lock screen when a scheduled ceremony starts.
pub async fn show_overlay(app: &AppHandle, ceremony_id: u64) -> Result<()> {
    update_active_overlay(app, ceremony_id, "showCeremonyOverlay").await
}

/// Tells Android that the webview has drawn the overlay content.
pub async fn mark_ready(app: &AppHandle, ceremony_id: u64) -> Result<()> {
    update_active_overlay(app, ceremony_id, "markCeremonyOverlayReady").await
}

/// Removes the notification and restores normal lock-screen behavior.
pub async fn end_overlay(ceremony_id: u64) -> Result<()> {
    let _guard = OVERLAY_COMMANDS.lock().await;
    super::bridge::call_async(
        "endCeremonyOverlay",
        json!({ "ceremonyId": ceremony_id.to_string() }),
    )
    .await
}
