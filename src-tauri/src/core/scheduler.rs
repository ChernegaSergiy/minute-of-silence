//! Ceremony scheduler and execution logic.

use chrono::{Datelike, FixedOffset, Local, NaiveDate, NaiveTime, Offset, TimeZone, Timelike, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::core::CeremonyManager;
use crate::core::audio::AudioEngine;
use crate::core::settings::{AnnouncementVoice, AudioPreset};
use crate::state::AppState;

/// Scheduler for the daily ceremony.
pub struct CeremonyScheduler {
    app: AppHandle,
    audio: Arc<AudioEngine>,
    voice_durations: HashMap<AnnouncementVoice, Duration>,
    bell_duration: Duration,
}

impl CeremonyScheduler {
    pub fn new(app: AppHandle) -> Self {
        let audio = app.state::<AppState>().audio.clone();
        let bell_duration = audio.get_duration("bell.ogg").unwrap_or(Duration::ZERO);

        let mut voice_durations = HashMap::new();
        for voice in [
            AnnouncementVoice::BohdanHdal,
            AnnouncementVoice::SoniaSotnyk,
            AnnouncementVoice::DaniaKhomutovskyi,
            AnnouncementVoice::RadioBg,
            AnnouncementVoice::AirAlert,
        ] {
            let filename = match voice {
                AnnouncementVoice::BohdanHdal => "announcement.ogg".to_string(),
                AnnouncementVoice::SoniaSotnyk => "announcement_sotnyk.ogg".to_string(),
                AnnouncementVoice::DaniaKhomutovskyi => "announcement_khomutovskyi.ogg".to_string(),
                AnnouncementVoice::RadioBg => "announcement_radio_bg.ogg".to_string(),
                AnnouncementVoice::AirAlert => "announcement_air_alert.ogg".to_string(),
            };
            let duration = audio.get_duration(&filename).unwrap_or(Duration::ZERO);
            voice_durations.insert(voice, duration);
            log::info!(
                "Voice {:?} ({}) duration: {:.2}s",
                voice,
                filename,
                duration.as_secs_f32()
            );
        }
        log::info!("Bell duration: {:.2}s", bell_duration.as_secs_f32());
        Self {
            app,
            audio,
            voice_durations,
            bell_duration,
        }
    }

    fn get_default_duration(&self) -> Duration {
        *self
            .voice_durations
            .get(&AnnouncementVoice::BohdanHdal)
            .unwrap()
    }

    /// Run the main scheduler loop.
    pub async fn run(&self) {
        log::info!("Scheduler loop started");

        // Listen for resume-from-sleep event (Windows)
        let state = self.app.state::<AppState>();
        let ntp_service = state.ntp_service.clone();
        let app_emit_handle = self.app.clone();

        self.app.listen("resume-from-sleep", move |_| {
            log::info!("Resume-from-sleep event received, forcing NTP sync...");
            let ntp = ntp_service.clone();
            let app = app_emit_handle.clone();
            tauri::async_runtime::spawn(async move {
                let _ = ntp.sync().await;
                let _ = app.emit("ntp-synced", ());
            });
        });

        // Initial NTP sync
        self.sync_ntp().await;

        // Track the date for which we already sent a reminder,
        // so we don't fire it again on the same day even after restart.
        let mut last_reminded_date: Option<NaiveDate> = None;

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            interval.tick().await;

            // Periodic NTP sync (every hour)
            let now_local = Local::now();
            if now_local.minute() == 0 && now_local.second() == 0 {
                self.sync_ntp().await;
            }

            let now = self.get_synchronized_now();
            let today = now.date_naive();
            let now_time = now.time();

            // Reminder notification
            let reminder_info = {
                let state = self.app.state::<AppState>();
                let inner = state.lock();
                let mins = inner.settings.reminder_minutes_before;

                if !inner.settings.reminder_enabled
                    || !inner.settings.ceremony_enabled
                    || (inner.settings.weekdays_only && today.weekday().number_from_monday() > 5)
                    || inner.settings.skip_date == Some(today)
                    || inner.last_activation.map(|dt| dt.date_naive()) == Some(today)
                    || last_reminded_date == Some(today)
                {
                    None
                } else {
                    let ceremony_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
                    // For "immediately" (mins == 0), remind_at is 09:00
                    // For scheduled reminders (mins > 0), remind_at is before 09:00
                    let remind_at = if mins == 0 {
                        ceremony_time
                    } else {
                        ceremony_time - chrono::Duration::minutes(mins as i64)
                    };

                    let fire = now_time.hour() == remind_at.hour()
                        && now_time.minute() == remind_at.minute()
                        && now_time.second() == 0;

                    if fire { Some(mins) } else { None }
                }
            };

            if let Some(mins) = reminder_info {
                last_reminded_date = Some(today);
                self.send_reminder_notification(mins);
            }

            // Ceremony trigger
            let should_trigger = {
                let state = self.app.state::<AppState>();
                let inner = state.lock();

                if !inner.settings.ceremony_enabled
                    || (inner.settings.weekdays_only && today.weekday().number_from_monday() > 5)
                {
                    false
                } else if !inner.ceremony_active {
                    let grace_minutes = inner.settings.late_start_grace_minutes;
                    let ceremony_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();

                    // Check if already triggered today
                    let last_activated = inner.last_activation.map(|dt| dt.date_naive());
                    if last_activated == Some(today) || inner.settings.skip_date == Some(today) {
                        false
                    } else {
                        let voice = inner.settings.announcement_voice;
                        let voice_duration = *self
                            .voice_durations
                            .get(&voice)
                            .unwrap_or(&self.get_default_duration());
                        let compensation = Self::get_compensation_duration(
                            inner.settings.preset,
                            voice_duration,
                            self.bell_duration,
                        );
                        // Check compensation window first: [09:00 - duration, 09:00)
                        let window_start = ceremony_time
                            - chrono::Duration::seconds(compensation.as_secs() as i64);
                        let in_compensation_window =
                            now_time >= window_start && now_time < ceremony_time;

                        // Also check grace window: [09:00, 09:00 + grace)
                        let in_grace_window =
                            self.is_within_window(now_time, ceremony_time, grace_minutes);

                        let should = in_compensation_window || in_grace_window;
                        if should && now_time.hour() == 8 && now_time.minute() == 59 {
                            log::info!(
                                "Trigger check: preset={:?}, compensation={:.2}s, now={}, window_start={}, in_compensation={}, in_grace={}, should_trigger={}",
                                inner.settings.preset,
                                compensation.as_secs_f32(),
                                now_time,
                                window_start,
                                in_compensation_window,
                                in_grace_window,
                                should
                            );
                        }
                        should
                    }
                } else {
                    false
                }
            };

            if should_trigger {
                log::info!("Ceremony triggered at {}", now_time);
                self.trigger_ceremony(false).await;
            }
        }
    }

    /// Send a system notification about the upcoming ceremony.
    fn send_reminder_notification(&self, mins_before: u8) {
        use rust_i18n::t;
        let title = t!("notification_title").to_string();
        let body = if mins_before == 0 {
            t!("notification_body_start").to_string()
        } else {
            t!("notification_body_reminder", mins => mins_before).to_string()
        };

        #[cfg(target_os = "windows")]
        if crate::platform::is_msix() {
            match crate::platform::windows::notifications::send_toast(&title, &body) {
                Ok(_) => log::info!(
                    "Reminder notification sent via WinRT ({} min before)",
                    mins_before
                ),
                Err(e) => log::warn!("MSIX toast failed: {e}"),
            }
            return;
        }

        // Standard path for non-MSIX Windows and Linux
        let result = self
            .app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show();

        match result {
            Ok(_) => log::info!("Reminder notification sent ({} min before)", mins_before),
            Err(e) => log::warn!("Failed to send reminder notification: {e}"),
        }
    }

    async fn sync_ntp(&self) {
        let state = self.app.state::<AppState>();

        if state.lock().settings.system_time_only {
            state.ntp_service.clear_cache();
            return;
        }

        log::info!("Attempting NTP synchronization...");
        let _ = state.ntp_service.sync().await;
        // Notify frontend (it will call get_status and use NtpService's state)
        let _ = self.app.emit("ntp-synced", ());
    }

    fn is_within_window(&self, now: NaiveTime, target: NaiveTime, grace_minutes: u8) -> bool {
        if now < target {
            return false;
        }
        let elapsed_secs = (now - target).num_seconds();
        elapsed_secs <= (grace_minutes as i64) * 60
    }

    fn preset_has_announcement(preset: AudioPreset) -> bool {
        matches!(
            preset,
            AudioPreset::VoiceMetronome
                | AudioPreset::VoiceSilenceBell
                | AudioPreset::VoiceSilence
                | AudioPreset::VoiceMetronomeAnthem
                | AudioPreset::VoiceMetronomeEnding
        )
    }

    fn preset_has_bell(preset: AudioPreset) -> bool {
        matches!(
            preset,
            AudioPreset::BellSilenceBell | AudioPreset::BellMetronomeBell
        )
    }

    fn get_compensation_duration(
        preset: AudioPreset,
        announcement_duration: Duration,
        bell_duration: Duration,
    ) -> Duration {
        if Self::preset_has_announcement(preset) {
            announcement_duration
        } else if Self::preset_has_bell(preset) {
            bell_duration
        } else {
            Duration::ZERO
        }
    }

    fn get_synchronized_now(&self) -> chrono::DateTime<FixedOffset> {
        let utc_now = Utc::now();

        let state = self.app.state::<AppState>();
        let ntp_offset = state.ntp_service.get_offset();
        let corrected_utc = if let Some(offset_ms) = ntp_offset {
            utc_now + chrono::Duration::milliseconds(offset_ms)
        } else {
            utc_now
        };

        corrected_utc.with_timezone(&Self::detect_local_offset())
    }

    fn detect_local_offset() -> FixedOffset {
        if let Ok(link) = std::fs::read_link("/etc/localtime") {
            let path_str = link.to_string_lossy();
            if let Some(tz_name) = path_str.rsplit("/zoneinfo/").next() {
                let tz_name = tz_name.trim();
                if !tz_name.is_empty() {
                    if let Ok(tz) = tz_name.parse::<chrono_tz::Tz>() {
                        let now = Utc::now();
                        return tz.offset_from_utc_datetime(&now.naive_utc()).fix();
                    }
                }
            }
        }

        *Local::now().offset()
    }

    pub async fn trigger_ceremony(&self, is_test: bool) {
        let platform = crate::platform::get_platform();
        let manager = CeremonyManager::new(self.app.clone(), platform, Arc::clone(&self.audio));
        manager.run_ceremony(is_test).await;
    }
}

pub async fn run(app: AppHandle) {
    let scheduler = CeremonyScheduler::new(app);
    scheduler.run().await;
}

pub async fn trigger_now(app: AppHandle, is_test: bool) {
    let scheduler = CeremonyScheduler::new(app);
    scheduler.trigger_ceremony(is_test).await;
}
