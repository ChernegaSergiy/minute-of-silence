//! System volume and mute state of the media stream.

use super::device;
use crate::error::Result;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct VolumeResponse {
    level: u8,
}

#[derive(Deserialize)]
struct MuteResponse {
    muted: bool,
}

/// Returns the media volume in percent.
pub fn get_volume() -> Result<u8> {
    let response: VolumeResponse = device::call("getVolume", json!({}))?;
    Ok(response.level.min(100))
}

/// Sets the media volume in percent.
pub fn set_volume(level: u8) -> Result<()> {
    device::call("setVolume", json!({ "level": level.min(100) }))
}

/// Returns whether the media stream is muted.
pub fn is_muted() -> Result<bool> {
    let response: MuteResponse = device::call("isMuted", json!({}))?;
    Ok(response.muted)
}

/// Mutes or unmutes the media stream.
pub fn set_mute(mute: bool) -> Result<()> {
    device::call("setMute", json!({ "mute": mute }))
}
