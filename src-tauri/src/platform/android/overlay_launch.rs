//! Coordinates full-screen ceremony launches with the Android activity.

use crate::error::Result;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlayLaunchState {
    launched_for_ceremony_overlay: bool,
}

/// Returns whether Android opened the activity for the ceremony overlay.
pub async fn is_overlay_launch() -> Result<bool> {
    let state: OverlayLaunchState =
        super::bridge::call_async("isCeremonyOverlayLaunch", json!({})).await?;
    Ok(state.launched_for_ceremony_overlay)
}

/// Finishes the activity if it was opened for the ceremony overlay.
pub async fn finish_overlay_launch() -> Result<()> {
    super::bridge::call_async("finishCeremonyOverlayLaunch", json!({})).await
}
