//! Coordinates full-screen ceremony launches with the Android activity.

use crate::error::Result;
use crate::state::{CeremonyKind, CeremonyPhase, CeremonyRuntime};
use serde_json::json;
use tokio::sync::Mutex;

static OVERLAY_COMMANDS: Mutex<()> = Mutex::const_new(());

async fn update_active_overlay(
    runtime: &CeremonyRuntime,
    ceremony_id: u64,
    command: &str,
) -> Result<()> {
    let _guard = OVERLAY_COMMANDS.lock().await;
    let should_update = {
        let inner = runtime.lock();
        inner.settings.show_visual_overlay
            && inner.active_ceremony.is_some_and(|active| {
                active.id == ceremony_id
                    && active.kind == CeremonyKind::Scheduled
                    && active.phase != CeremonyPhase::Finishing
            })
    };
    if should_update {
        super::device::call::<()>(command, json!({ "ceremonyId": ceremony_id.to_string() }))?;
    }
    Ok(())
}

/// Shows the overlay over the lock screen when a scheduled ceremony starts.
pub async fn show_overlay(runtime: &CeremonyRuntime, ceremony_id: u64) -> Result<()> {
    update_active_overlay(runtime, ceremony_id, "showCeremonyOverlay").await
}

/// Tells Android that the webview has drawn the overlay content.
pub async fn mark_ready(runtime: &CeremonyRuntime, ceremony_id: u64) -> Result<()> {
    update_active_overlay(runtime, ceremony_id, "markCeremonyOverlayReady").await
}

/// Removes the notification and restores normal lock-screen behavior.
pub async fn end_overlay(ceremony_id: u64) -> Result<()> {
    let _guard = OVERLAY_COMMANDS.lock().await;
    super::device::call(
        "endCeremonyOverlay",
        json!({ "ceremonyId": ceremony_id.to_string() }),
    )
}
