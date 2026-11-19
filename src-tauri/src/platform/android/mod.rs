pub mod audio;
mod autostart;
pub mod bridge;
pub mod custom_tabs;
pub mod device;
pub mod headless;
pub(crate) mod media;
pub mod notification_actions;
pub mod permission;
pub mod schedule;
mod volume;

use crate::error::Result;

pub struct AndroidPlatform;

pub fn apply_autostart_enabled(_app: &tauri::AppHandle, enabled: bool) {
    if let Err(e) = autostart::set_enabled(enabled) {
        log::warn!("Failed to apply the autostart setting: {}", e);
    }
}

impl super::Platform for AndroidPlatform {
    fn get_volume(&self) -> Result<u8> {
        volume::get_volume()
    }

    fn set_volume(&self, level: u8) -> Result<()> {
        volume::set_volume(level)
    }

    fn is_muted(&self) -> Result<bool> {
        volume::is_muted()
    }

    fn set_mute(&self, mute: bool) -> Result<()> {
        volume::set_mute(mute)
    }
}
