//! Linux volume control using native ALSA API.

use crate::error::{AppError, Result};
use alsa::mixer::{Mixer, Selem, SelemId};

pub fn get_volume() -> Result<u8> {
    let mixer = open_mixer()?;
    let selem = find_master_selem(&mixer)?;

    let (min, max) = selem.get_playback_volume_range();
    let volume = selem
        .get_playback_volume(alsa::mixer::SelemChannelId::FrontLeft)
        .map_err(|e| AppError::Platform(e.to_string()))?;

    if max == min {
        return Ok(0);
    }

    let percent = ((volume - min) as f64 / (max - min) as f64 * 100.0) as u8;
    Ok(percent)
}

pub fn set_volume(level: u8) -> Result<()> {
    let mixer = open_mixer()?;
    let selem = find_master_selem(&mixer)?;

    let (min, max) = selem.get_playback_volume_range();
    let val = (min as f64 + (max - min) as f64 * (level as f64 / 100.0)) as i64;

    selem
        .set_playback_volume_all(val)
        .map_err(|e| AppError::Platform(e.to_string()))?;

    Ok(())
}

pub fn is_muted() -> Result<bool> {
    let mixer = open_mixer()?;
    let selem = find_master_selem(&mixer)?;

    let switch = selem
        .get_playback_switch(alsa::mixer::SelemChannelId::FrontLeft)
        .map_err(|e| AppError::Platform(e.to_string()))?;

    Ok(switch == 0)
}

pub fn set_mute(mute: bool) -> Result<()> {
    let mixer = open_mixer()?;
    let selem = find_master_selem(&mixer)?;

    let switch = if mute { 0 } else { 1 };
    selem
        .set_playback_switch_all(switch)
        .map_err(|e| AppError::Platform(e.to_string()))?;

    Ok(())
}

fn open_mixer() -> Result<Mixer> {
    Mixer::new("default", false).map_err(|e| AppError::Platform(e.to_string()))
}

fn find_master_selem(mixer: &Mixer) -> Result<Selem<'_>> {
    let sid = SelemId::new("Master", 0);
    mixer
        .find_selem(&sid)
        .ok_or_else(|| AppError::Platform("Could not find 'Master' mixer element".into()))
}
