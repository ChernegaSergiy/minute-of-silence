//! Ceremony scheduler and execution logic.

use chrono::{Datelike, FixedOffset, Local, NaiveDate, NaiveTime, Offset, TimeZone, Timelike, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
#[cfg(target_os = "windows")]
use tauri::Listener;
use tauri::{AppHandle, Emitter, Manager};
#[cfg(not(target_os = "android"))]
use tauri_plugin_notification::NotificationExt;

use crate::core::CeremonyManager;
#[cfg(any(target_os = "android", test))]
use crate::core::settings::PersonalDate;
use crate::core::settings::{AnnouncementVoice, PresetBlock};
use crate::core::voices::effective_blocks;
#[cfg(target_os = "android")]
use crate::state::CeremonyKind;
use crate::state::{AppState, CeremonyRuntime};

/// Scheduler for the daily ceremony.
pub struct CeremonyScheduler {
    runtime: Arc<CeremonyRuntime>,
    voice_durations: HashMap<AnnouncementVoice, Duration>,
    bell_durations: HashMap<AnnouncementVoice, Duration>,
}

impl CeremonyScheduler {
    pub(crate) fn new(runtime: Arc<CeremonyRuntime>) -> Self {
        let audio = Arc::clone(&runtime.audio);
        let mut voice_durations = HashMap::new();
        let mut bell_durations = HashMap::new();
        for voice in [
            AnnouncementVoice::BohdanHdal,
            AnnouncementVoice::SoniaSotnyk,
            AnnouncementVoice::DaniaKhomutovskyi,
            AnnouncementVoice::RadioBg,
            AnnouncementVoice::VinnytsiaCity,
        ] {
            let filename = audio.get_announcement_filename(voice);
            let duration = audio.get_duration(&filename).unwrap_or(Duration::ZERO);
            voice_durations.insert(voice, duration);
            log::info!(
                "Voice {:?} ({}) duration: {:.2}s",
                voice,
                filename,
                duration.as_secs_f32()
            );

            let bell_file = audio.get_bell_filename(voice);
            let bell_duration = audio.get_duration(&bell_file).unwrap_or(Duration::ZERO);
            bell_durations.insert(voice, bell_duration);
            log::info!(
                "Voice {:?} bell ({}) duration: {:.2}s",
                voice,
                bell_file,
                bell_duration.as_secs_f32()
            );
        }
        Self {
            runtime,
            voice_durations,
            bell_durations,
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

        #[cfg(target_os = "windows")]
        if let Some(app) = self.runtime.app() {
            let runtime = Arc::clone(&self.runtime);
            app.listen("resume-from-sleep", move |_| {
                let runtime = Arc::clone(&runtime);
                tauri::async_runtime::spawn(async move { Self::sync_ntp(&runtime).await });
            });
        }

        // Initial NTP sync
        #[cfg(not(target_os = "android"))]
        Self::sync_ntp(&self.runtime).await;
        #[cfg(target_os = "android")]
        self.sync_ntp_in_background();

        // Track the date for which we already sent a reminder,
        // so we don't fire it again on the same day even after restart.
        let mut last_reminded_date: Option<NaiveDate> = None;

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            interval.tick().await;

            // Periodic NTP sync (every hour)
            let now_local = Local::now();
            if now_local.minute() == 0 && now_local.second() == 0 {
                #[cfg(not(target_os = "android"))]
                Self::sync_ntp(&self.runtime).await;
                #[cfg(target_os = "android")]
                self.sync_ntp_in_background();
            }

            let now = self.get_synchronized_now();
            let today = now.date_naive();
            let now_time = now.time();

            // Reminder notification
            let reminder_info = {
                let inner = self.runtime.lock();
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
                let inner = self.runtime.lock();

                if !inner.settings.ceremony_enabled
                    || (inner.settings.weekdays_only && today.weekday().number_from_monday() > 5)
                {
                    false
                } else if inner.active_ceremony.is_none() {
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
                        let bell_duration = self
                            .bell_durations
                            .get(&voice)
                            .copied()
                            .unwrap_or(Duration::ZERO);
                        let blocks = effective_blocks(&inner.settings.blocks, voice);
                        let compensation =
                            Self::get_compensation_duration(&blocks, voice_duration, bell_duration);
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
                                "Trigger check: blocks={:?}, compensation={:.2}s, now={}, window_start={}, in_compensation={}, in_grace={}, should_trigger={}",
                                blocks,
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
                #[cfg(target_os = "android")]
                self.schedule_personal_dates_notification(today, now_time);
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

        #[cfg(target_os = "android")]
        let result = crate::platform::android::device::call::<()>(
            "postReminder",
            serde_json::json!({ "title": title, "body": body }),
        );
        #[cfg(not(target_os = "android"))]
        let result = {
            let Some(app) = self.runtime.app() else {
                return;
            };
            app.notification().builder().title(title).body(body).show()
        };

        match result {
            Ok(_) => log::info!("Reminder notification sent ({} min before)", mins_before),
            Err(e) => log::warn!("Failed to send reminder notification: {e}"),
        }
    }

    #[cfg(target_os = "android")]
    fn schedule_personal_dates_notification(&self, date: NaiveDate, time: NaiveTime) {
        let dates_result = match self.runtime.app() {
            Some(app) => crate::app::commands::get_personal_dates(app),
            None => crate::platform::android::headless::read_personal_dates(),
        };
        let dates = match dates_result {
            Ok(dates) => dates,
            Err(error) => {
                log::warn!("Failed to load personal dates for notification: {error}");
                return;
            }
        };
        let labels = personal_date_labels_for_day(&dates, date);
        if labels.is_empty() {
            return;
        }

        let ceremony_time = NaiveTime::MIN + chrono::Duration::hours(9);
        let delay = ceremony_time
            .signed_duration_since(time)
            .to_std()
            .unwrap_or_default();
        if delay.is_zero() {
            if let Err(error) = crate::platform::android::device::call::<()>(
                "postPersonalDates",
                serde_json::json!({ "labels": labels }),
            ) {
                log::warn!("Failed to post personal dates notification: {error}");
            }
            return;
        }

        let runtime = Arc::clone(&self.runtime);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(delay).await;

            let ceremony_is_active = runtime
                .lock()
                .active_ceremony
                .is_some_and(|ceremony| ceremony.kind == CeremonyKind::Scheduled);
            if !ceremony_is_active {
                return;
            }

            if let Err(error) = crate::platform::android::device::call::<()>(
                "postPersonalDates",
                serde_json::json!({ "labels": labels }),
            ) {
                log::warn!("Failed to post personal dates notification: {error}");
            }
        });
    }

    #[cfg(target_os = "android")]
    fn sync_ntp_in_background(&self) {
        let runtime = Arc::clone(&self.runtime);
        tauri::async_runtime::spawn(async move { Self::sync_ntp(&runtime).await });
    }

    async fn sync_ntp(runtime: &CeremonyRuntime) {
        if runtime.lock().settings.system_time_only {
            runtime.ntp_service.clear_cache();
            return;
        }

        log::info!("Attempting NTP synchronization...");
        let _ = runtime.ntp_service.sync().await;
        // Notify frontend (it will call get_status and use NtpService's state)
        if let Some(app) = runtime.app() {
            let _ = app.emit("ntp-synced", ());
        }
    }

    fn is_within_window(&self, now: NaiveTime, target: NaiveTime, grace_minutes: u8) -> bool {
        if now < target {
            return false;
        }
        let elapsed_secs = (now - target).num_seconds();
        elapsed_secs <= (grace_minutes as i64) * 60
    }

    /// How long before 09:00 the ceremony has to start so that the main part
    /// (the metronome or the silence) begins exactly at 09:00.
    ///
    /// That is the length of the announcement and bells that open the preset,
    /// plus the short pause that separates them from the metronome.
    pub(crate) fn get_compensation_duration(
        blocks: &[PresetBlock],
        announcement_duration: Duration,
        bell_duration: Duration,
    ) -> Duration {
        khvylyna_core::scheduler::get_compensation_duration(
            blocks,
            announcement_duration,
            bell_duration,
        )
    }

    fn get_synchronized_now(&self) -> chrono::DateTime<FixedOffset> {
        let utc_now = Utc::now();

        let ntp_offset = self.runtime.ntp_service.get_offset();
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
        let manager = CeremonyManager::new(Arc::clone(&self.runtime), platform);
        manager.run_ceremony(is_test).await;
    }
}

#[cfg(any(target_os = "android", test))]
fn personal_date_labels_for_day(dates: &[PersonalDate], date: NaiveDate) -> Vec<String> {
    let month = date.month() as u8;
    let day = date.day() as u8;
    let is_non_leap_february_28 = month == 2 && day == 28 && date.with_day(29).is_none();

    dates
        .iter()
        .filter(|personal_date| {
            (personal_date.month == month && personal_date.day == day)
                || (is_non_leap_february_28 && personal_date.month == 2 && personal_date.day == 29)
        })
        .map(|personal_date| personal_date.label.clone())
        .collect()
}

pub(crate) async fn run(runtime: Arc<CeremonyRuntime>) {
    let scheduler = CeremonyScheduler::new(runtime);
    scheduler.run().await;
}

pub async fn trigger_now(app: AppHandle, is_test: bool) {
    let runtime = Arc::clone(&app.state::<AppState>().runtime);
    let platform = crate::platform::get_platform();
    CeremonyManager::new(runtime, platform)
        .run_ceremony(is_test)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::AudioPreset::{self, *};
    use khvylyna_core::scheduler::METRONOME_LEAD_PAUSE;

    const ANNOUNCEMENT: Duration = Duration::from_secs(40);
    const BELL: Duration = Duration::from_secs(9);

    fn compensation(preset: AudioPreset) -> Duration {
        CeremonyScheduler::get_compensation_duration(preset.blocks(), ANNOUNCEMENT, BELL)
    }

    #[test]
    fn presets_that_open_with_the_announcement_start_early_by_its_length() {
        for preset in [VoiceSilenceBell, VoiceSilence] {
            assert_eq!(compensation(preset), ANNOUNCEMENT, "{preset:?}");
        }
    }

    #[test]
    fn presets_that_open_with_a_bell_start_early_by_its_length() {
        assert_eq!(compensation(BellSilenceBell), BELL);
    }

    #[test]
    fn the_pause_before_the_metronome_starts_the_ceremony_one_second_earlier() {
        for preset in [VoiceMetronome, VoiceMetronomeAnthem, VoiceMetronomeEnding] {
            assert_eq!(
                compensation(preset),
                ANNOUNCEMENT + METRONOME_LEAD_PAUSE,
                "{preset:?}"
            );
        }
        assert_eq!(compensation(BellMetronomeBell), BELL + METRONOME_LEAD_PAUSE);
    }

    #[test]
    fn presets_without_an_opening_sound_do_not_start_early() {
        for preset in [MetronomeOnly, MetronomeAnthem, Silence] {
            assert_eq!(compensation(preset), Duration::ZERO, "{preset:?}");
        }
    }

    #[test]
    fn a_bell_built_into_the_announcement_is_not_counted_twice() {
        use PresetBlock::*;

        let blocks = effective_blocks(
            &[Bell, Announcement, Metronome],
            AnnouncementVoice::VinnytsiaCity,
        );
        assert_eq!(
            CeremonyScheduler::get_compensation_duration(&blocks, ANNOUNCEMENT, BELL),
            ANNOUNCEMENT + METRONOME_LEAD_PAUSE
        );
    }

    #[test]
    fn a_bell_and_an_announcement_both_count_towards_the_lead_in() {
        use PresetBlock::*;

        let blocks = [Bell, Announcement, Metronome, Bell];
        assert_eq!(
            CeremonyScheduler::get_compensation_duration(&blocks, ANNOUNCEMENT, BELL),
            ANNOUNCEMENT + BELL + METRONOME_LEAD_PAUSE
        );
    }

    #[test]
    fn personal_dates_for_today_include_all_matching_labels() {
        let dates = [
            PersonalDate {
                id: None,
                month: 11,
                day: 19,
                year: 2014,
                label: "Date one".to_string(),
            },
            PersonalDate {
                id: None,
                month: 11,
                day: 19,
                year: 2020,
                label: "Date two".to_string(),
            },
            PersonalDate {
                id: None,
                month: 11,
                day: 20,
                year: 2010,
                label: "Other date".to_string(),
            },
        ];

        assert_eq!(
            personal_date_labels_for_day(
                &dates,
                NaiveDate::from_ymd_opt(2026, 11, 19).expect("valid date")
            ),
            ["Date one", "Date two"]
        );
    }

    #[test]
    fn february_29_dates_are_included_on_february_28_in_non_leap_years() {
        let date = PersonalDate {
            id: None,
            month: 2,
            day: 29,
            year: 2000,
            label: "Leap day".to_string(),
        };

        assert_eq!(
            personal_date_labels_for_day(
                std::slice::from_ref(&date),
                NaiveDate::from_ymd_opt(2025, 2, 28).expect("valid date")
            ),
            ["Leap day"]
        );
        assert!(
            personal_date_labels_for_day(
                std::slice::from_ref(&date),
                NaiveDate::from_ymd_opt(2024, 2, 28).expect("valid date")
            )
            .is_empty()
        );
    }
}
