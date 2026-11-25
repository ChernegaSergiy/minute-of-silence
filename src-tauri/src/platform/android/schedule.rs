//! The background wake-up alarm before the ceremony.

use super::bridge;
use crate::Settings;
use crate::error::{AppError, Result};
use serde_json::json;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static CONFIGURATION_GENERATION: AtomicU64 = AtomicU64::new(0);
static CONFIGURATION_CALL: Mutex<()> = Mutex::new(());

/// Reserves an ordering token for a desired native schedule configuration.
pub fn next_configuration_generation() -> u64 {
    CONFIGURATION_GENERATION.fetch_add(1, Ordering::AcqRel) + 1
}

/// Tells the native side when to wake the device for the ceremony.
///
/// The Rust scheduler stays the one that decides whether a ceremony runs.
/// Applies a configuration only if no newer settings update superseded it.
pub fn configure(settings: &Settings, generation: u64) -> Result<()> {
    let _guard = CONFIGURATION_CALL
        .lock()
        .map_err(|_| AppError::Platform("Android schedule configuration lock poisoned".into()))?;
    if CONFIGURATION_GENERATION.load(Ordering::Acquire) != generation {
        return Ok(());
    }

    bridge::call(
        "configureSchedule",
        json!({
            "enabled": settings.ceremony_enabled,
            "weekdaysOnly": settings.weekdays_only,
            "skipDate": settings.skip_date.map(|date| date.format("%Y-%m-%d").to_string()),
        }),
    )
}
