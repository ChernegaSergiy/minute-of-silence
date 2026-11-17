//! Commands of the native audio player.

use super::bridge;
use crate::error::Result;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Deserialize)]
struct IdleResponse {
    idle: bool,
}

#[derive(Deserialize)]
struct DurationResponse {
    #[serde(rename = "durationMs")]
    duration_ms: u64,
}

/// Queues a bundled recording behind the ones already queued.
///
/// `volume` is the player gain from 0.0 to 1.0.
pub fn enqueue(name: &str, volume: f32) -> Result<()> {
    bridge::call("enqueueAudio", json!({ "name": name, "volume": volume }))
}

/// Returns true when nothing plays and nothing is queued.
pub fn is_idle() -> Result<bool> {
    let response: IdleResponse = bridge::call("isAudioIdle", json!({}))?;
    Ok(response.idle)
}

/// Stops the playback and drops the queue.
pub fn stop() -> Result<()> {
    bridge::call("stopAudio", json!({}))
}

/// Returns the length of a bundled recording.
pub fn duration(name: &str) -> Result<Duration> {
    let response: DurationResponse = bridge::call("getAudioDuration", json!({ "name": name }))?;
    Ok(Duration::from_millis(response.duration_ms))
}
