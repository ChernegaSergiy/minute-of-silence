pub mod audio;
mod autostart;
pub mod bridge;
mod media;
pub mod permission;
pub mod schedule;
mod volume;

use crate::error::Result;

pub struct AndroidPlatform;

pub fn is_dark_mode() -> bool {
    false
}

pub fn system_autostart_enabled() -> Option<bool> {
    None
}

pub fn apply_autostart_enabled(_app: &tauri::AppHandle, enabled: bool) {
    if let Err(e) = autostart::set_enabled(enabled) {
        log::warn!("Failed to apply the autostart setting: {}", e);
    }
}

#[async_trait::async_trait]
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

    async fn pause_media(&self) -> Result<Vec<String>> {
        media::pause_media().await
    }

    async fn resume_media(&self, players: Vec<String>) -> Result<()> {
        media::resume_media(players).await
    }
}
