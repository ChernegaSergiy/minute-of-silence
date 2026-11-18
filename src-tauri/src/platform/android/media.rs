//! Pausing and resuming other players through the audio focus.

use super::device;
use crate::error::Result;
use serde::Deserialize;
use serde_json::json;

/// Marker returned by [`pause_media`] while the audio focus is held.
const FOCUS_MARKER: &str = "audio-focus";

#[derive(Deserialize)]
struct PauseResponse {
    granted: bool,
}

/// Takes the audio focus so that other players pause.
///
/// The returned list is empty when Android refused the focus, which tells the
/// ceremony that there is nothing to resume.
pub async fn pause_media() -> Result<Vec<String>> {
    let response: PauseResponse = device::call("pauseMedia", json!({}))?;
    if response.granted {
        Ok(vec![FOCUS_MARKER.to_string()])
    } else {
        log::warn!("Android did not grant the audio focus, other players keep playing");
        Ok(Vec::new())
    }
}

/// Releases the audio focus so that the paused players can resume.
pub async fn resume_media(players: Vec<String>) -> Result<()> {
    if players.is_empty() {
        return Ok(());
    }
    device::call("resumeMedia", json!({}))
}
