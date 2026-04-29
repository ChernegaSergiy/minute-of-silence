//! Pause / resume system-wide media playback on Windows.

use log::{error, info};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackInfo,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

use crate::error::{AppError, Result};

pub async fn pause_all() -> Result<()> {
    let manager: GlobalSystemMediaTransportControlsSessionManager =
        GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?
            .await
            .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?;

    let sessions = manager
        .GetSessions()
        .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?;

    let count = sessions
        .Size()
        .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?;

    info!("Found {} media sessions", count);

    for i in 0..count {
        if let Ok(session) = sessions.GetAt(i) {
            let app_id: windows::core::HSTRING = session.SourceAppUserModelId().unwrap_or_default();
            info!("Session {}: AppId={}", i, app_id);

            let playback_info: GlobalSystemMediaTransportControlsSessionPlaybackInfo =
                match session.GetPlaybackInfo() {
                    Ok(info) => info,
                    Err(e) => {
                        error!("Failed to get playback info for session {}: {:?}", i, e);
                        continue;
                    }
                };

            let status = match playback_info.PlaybackStatus() {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to get playback status for session {}: {:?}", i, e);
                    continue;
                }
            };

            if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                info!("Pausing session {}...", i);
                let _ = session.TryPauseAsync();
            }
        }
    }

    Ok(())
}
