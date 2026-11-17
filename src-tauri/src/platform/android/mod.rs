pub mod bridge;
mod volume;

use crate::error::{AppError, Result};

pub struct AndroidPlatform;

pub fn is_dark_mode() -> bool {
    false
}

pub fn system_autostart_enabled() -> Option<bool> {
    None
}

pub fn apply_autostart_enabled(_app: &tauri::AppHandle, _enabled: bool) {}

fn unsupported<T>() -> Result<T> {
    Err(AppError::Platform(
        "Android platform integration is not implemented yet".to_string(),
    ))
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
        unsupported()
    }

    async fn resume_media(&self, _players: Vec<String>) -> Result<()> {
        unsupported()
    }
}
