//! The wake-up alarm that brings the app up before the ceremony.

use super::bridge;
use crate::Settings;
use crate::error::Result;
use serde_json::json;

/// Tells the native side when the ceremony is due so that it can wake the app.
///
/// The Rust scheduler stays the one that decides whether a ceremony runs.
pub fn configure(settings: &Settings) -> Result<()> {
    bridge::call(
        "configureSchedule",
        json!({
            "enabled": settings.ceremony_enabled,
            "weekdaysOnly": settings.weekdays_only,
            "skipDate": settings.skip_date.map(|date| date.format("%Y-%m-%d").to_string()),
            "showVisualOverlay": settings.show_visual_overlay,
        }),
    )
}
