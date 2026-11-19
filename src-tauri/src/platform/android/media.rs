//! Temporary Android audio focus owned by a ceremony.

use super::device;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct FocusResponse {
    granted: bool,
}

/// Requests transient focus for the current ceremony.
pub(crate) fn request_focus(ceremony_id: u64) {
    match device::call::<FocusResponse>(
        "requestAudioFocus",
        json!({ "ceremonyId": ceremony_id.to_string() }),
    ) {
        Ok(response) if response.granted => {}
        Ok(_) => log::warn!("Android did not grant audio focus for the ceremony"),
        Err(error) => log::warn!("Failed to request Android audio focus: {error}"),
    }
}

/// Releases focus only if this ceremony acquired it.
pub(crate) fn release_focus(ceremony_id: u64) {
    if let Err(error) = device::call::<()>(
        "releaseAudioFocus",
        json!({ "ceremonyId": ceremony_id.to_string() }),
    ) {
        log::warn!("Failed to release Android audio focus: {error}");
    }
}
