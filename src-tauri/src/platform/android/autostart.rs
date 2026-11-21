//! Autostart on Android.
//!
//! Android has no start at login. The closest thing is that the app sets its
//! wake-up alarm again after a reboot, and that is what the setting controls.

use super::bridge;
use crate::error::Result;
use serde_json::json;

/// Switches the alarm re-arming after a reboot on or off.
pub fn set_enabled(enabled: bool) -> Result<()> {
    tauri::async_runtime::spawn(async move {
        if let Err(error) =
            bridge::call_async::<()>("setAutostart", json!({ "enabled": enabled })).await
        {
            log::warn!("Failed to apply the autostart setting: {error}");
        }
    });
    Ok(())
}
