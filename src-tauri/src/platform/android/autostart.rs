//! Autostart on Android.
//!
//! Android has no start at login. The closest thing is that the app sets its
//! wake-up alarm again after a reboot, and that is what the setting controls.

use super::bridge;
use crate::error::Result;
use serde_json::json;

/// Switches the alarm re-arming after a reboot on or off.
pub fn set_enabled(enabled: bool) -> Result<()> {
    bridge::call("setAutostart", json!({ "enabled": enabled }))
}
