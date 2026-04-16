pub mod audio;
pub mod ntp;
pub mod ntp_service;
pub mod platform;
pub mod scheduler;
pub mod settings;

use crate::core::audio::AudioEngine;
use crate::core::platform::Platform;
use crate::core::settings::AudioPreset;
use crate::state::AppState;
use chrono::{Local, NaiveTime, Timelike};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

lazy_static::lazy_static! {
    static ref PREVIOUS_VOLUME: Mutex<Option<u8>> = Mutex::new(None);
    static ref WAS_MUTED: Mutex<Option<bool>> = Mutex::new(None);
}

/// Orchestrator for the ceremony.
/// This class manages the sequence of events during the ceremony.
pub struct CeremonyManager {
    app: AppHandle,
    platform: Box<dyn Platform>,
    audio: Arc<AudioEngine>,
}

impl CeremonyManager {
    pub fn new(app: AppHandle, platform: Box<dyn Platform>, audio: Arc<AudioEngine>) -> Self {
        Self {
            app,
            platform,
            audio,
        }
    }

    pub async fn run_ceremony(&self) {
        let (should_pause_players, volume_priority, auto_unmute, target_volume, preset) = {
            let state = self.app.state::<AppState>();
            let inner = state.lock();
            (
                inner.settings.pause_other_players,
                inner.settings.volume_priority,
                inner.settings.auto_unmute,
                inner.settings.volume,
                inner.settings.preset,
            )
        };

        // 1. Mark active
        {
            let state = self.app.state::<AppState>();
            let mut inner = state.lock();
            inner.ceremony_active = true;
            inner.last_activation = Some(chrono::Local::now());
        }

        // 2. Notify UI
        let _ = self.app.emit("ceremony-start", ());

        // 3. Pause players
        if should_pause_players {
            let _ = self.platform.pause_media().await;
        }

        // 4. Handle Volume and Mute (skip for Silence preset)
        if auto_unmute && preset != AudioPreset::Silence {
            // Save mute state and unmute if necessary
            if let Ok(muted) = self.platform.is_muted() {
                if muted {
                    *WAS_MUTED.lock().unwrap() = Some(true);
                    let _ = self.platform.set_mute(false);
                }
            }
        }

        if volume_priority && preset != AudioPreset::Silence {
            // Save and set volume
            if let Ok(vol) = self.platform.get_volume() {
                *PREVIOUS_VOLUME.lock().unwrap() = Some(vol);
                let _ = self.platform.set_volume(target_volume);
            }
        }

        // 6. Calculate timing for precise ceremony start
        let now = Local::now();
        let should_wait = Self::should_use_precise_timing(&now, preset);
        let wait_duration = if should_wait {
            self.calculate_timing_delay(preset)
        } else {
            Duration::ZERO
        };

        // 7. Play Audio (Stop previous first)
        self.audio.stop();

        let audio_engine = Arc::clone(&self.audio);
        let app_handle = self.app.clone();
        let platform_handle = platform::get_platform();

        std::thread::spawn(move || {
            if wait_duration > Duration::ZERO {
                log::info!(
                    "Waiting {:.0}s before audio for precise 09:00 start",
                    wait_duration.as_secs_f32()
                );
                std::thread::sleep(wait_duration);
            }

            if let Err(e) = audio_engine.play_preset(preset, target_volume) {
                log::error!("Ceremony audio error: {}", e);
            }

            // 8. Finish
            tauri::async_runtime::spawn(async move {
                CeremonyManager::finish_ceremony(app_handle, platform_handle).await;
            });
        });
    }

    fn should_use_precise_timing(now: &chrono::DateTime<Local>, preset: AudioPreset) -> bool {
        let time = now.time();
        let is_close_to_9am = time.hour() == 9 && time.minute() == 0 && time.second() <= 2;
        let has_announcement = matches!(
            preset,
            AudioPreset::VoiceMetronome
                | AudioPreset::VoiceSilenceBell
                | AudioPreset::VoiceSilence
                | AudioPreset::VoiceMetronomeAnthem
        );
        is_close_to_9am && has_announcement
    }

    fn calculate_timing_delay(&self, preset: AudioPreset) -> Duration {
        let now = Local::now();
        let target = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let announcement_duration = match Self::get_announcement_filename(preset) {
            Some(filename) => self.audio.get_duration(filename).unwrap_or(Duration::ZERO),
            None => Duration::ZERO,
        };

        let seconds_until_9am = if now.time() < target {
            (target - now.time()).num_seconds() as u64
        } else {
            0
        };

        if seconds_until_9am > announcement_duration.as_secs() {
            Duration::from_secs(seconds_until_9am - announcement_duration.as_secs())
        } else {
            Duration::ZERO
        }
    }

    fn get_announcement_filename(preset: AudioPreset) -> Option<&'static str> {
        match preset {
            AudioPreset::VoiceMetronome
            | AudioPreset::VoiceSilenceBell
            | AudioPreset::VoiceSilence
            | AudioPreset::VoiceMetronomeAnthem => Some("announcement.ogg"),
            _ => None,
        }
    }

    pub async fn finish_ceremony(app: AppHandle, platform: Box<dyn Platform>) {
        let (volume_priority, auto_unmute) = {
            let state = app.state::<AppState>();
            let inner = state.lock();
            if !inner.ceremony_active {
                return;
            }
            (inner.settings.volume_priority, inner.settings.auto_unmute)
        };

        {
            let state = app.state::<AppState>();
            let mut inner = state.lock();
            inner.ceremony_active = false;
        }

        // Restore volume and mute
        if volume_priority {
            let prev_vol = *PREVIOUS_VOLUME.lock().unwrap();
            if let Some(vol) = prev_vol {
                let _ = platform.set_volume(vol);
                *PREVIOUS_VOLUME.lock().unwrap() = None;
            }
        }

        if auto_unmute {
            let was_muted = *WAS_MUTED.lock().unwrap();
            if let Some(true) = was_muted {
                let _ = platform.set_mute(true);
                *WAS_MUTED.lock().unwrap() = None;
            }
        }

        let _ = app.emit("ceremony-end", ());
    }
}
