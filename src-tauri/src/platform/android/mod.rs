pub mod bridge;

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
        unsupported()
    }

    fn set_volume(&self, _level: u8) -> Result<()> {
        unsupported()
    }

    fn is_muted(&self) -> Result<bool> {
        unsupported()
    }

    fn set_mute(&self, _mute: bool) -> Result<()> {
        unsupported()
    }

    async fn pause_media(&self) -> Result<Vec<String>> {
        unsupported()
    }

    async fn resume_media(&self, _players: Vec<String>) -> Result<()> {
        unsupported()
    }
}
