//! Core scheduling, lead-in compensation, and trigger evaluation.
//!
//! Works in both `std` and `no_std + alloc` environments.

use core::time::Duration;

use crate::voices::{VoiceDurations, effective_blocks};
use crate::{PresetBlock, Settings};

/// Silence between a voice or a bell and the metronome that follows it.
pub const METRONOME_LEAD_PAUSE: Duration = Duration::from_secs(1);

/// Target ceremony hour (09:00:00).
pub const CEREMONY_HOUR: u8 = 9;
/// Target ceremony minute (09:00:00).
pub const CEREMONY_MINUTE: u8 = 0;
/// Target ceremony second (09:00:00).
pub const CEREMONY_SECOND: u8 = 0;

/// The pause that separates the opening announcement and bells from the metronome.
///
/// It is [`METRONOME_LEAD_PAUSE`] when the metronome directly follows them and
/// zero otherwise. Playback and start compensation both use this rule.
#[inline]
pub fn lead_in_pause(blocks: &[PresetBlock]) -> Duration {
    let lead_in_len = blocks
        .iter()
        .take_while(|block| matches!(block, PresetBlock::Announcement | PresetBlock::Bell))
        .count();

    if lead_in_len > 0 && blocks.get(lead_in_len) == Some(&PresetBlock::Metronome) {
        METRONOME_LEAD_PAUSE
    } else {
        Duration::ZERO
    }
}

/// Calculate how long before 09:00 the ceremony has to start so that the main part
/// (the metronome or the silence) begins exactly at 09:00.
///
/// That is the length of the announcement and bells that open the preset,
/// plus the short pause that separates them from the metronome.
pub fn get_compensation_duration(
    blocks: &[PresetBlock],
    announcement_duration: Duration,
    bell_duration: Duration,
) -> Duration {
    let lead_in_duration: Duration = blocks
        .iter()
        .take_while(|block| matches!(block, PresetBlock::Announcement | PresetBlock::Bell))
        .map(|block| match block {
            PresetBlock::Announcement => announcement_duration,
            _ => bell_duration,
        })
        .sum();

    lead_in_duration + lead_in_pause(blocks)
}

/// Calculate compensation duration directly from settings using known voice durations.
pub fn get_settings_compensation(
    settings: &Settings,
    voice_durations: &VoiceDurations,
) -> Duration {
    let voice = settings.announcement_voice;
    let blocks = effective_blocks(&settings.blocks, voice);
    get_compensation_duration(&blocks, voice_durations.announcement, voice_durations.bell)
}

/// Check if a time (hour, minute, second) falls in the lead-in compensation window:
/// `[09:00:00 - compensation, 09:00:00)`
pub fn is_in_compensation_window(hour: u8, minute: u8, second: u8, compensation: Duration) -> bool {
    let current_secs = (hour as u32) * 3600 + (minute as u32) * 60 + (second as u32);
    let target_secs =
        (CEREMONY_HOUR as u32) * 3600 + (CEREMONY_MINUTE as u32) * 60 + (CEREMONY_SECOND as u32);

    let comp_secs = compensation.as_secs() as u32;
    if comp_secs == 0 {
        return false;
    }

    let window_start = target_secs.saturating_sub(comp_secs);
    current_secs >= window_start && current_secs < target_secs
}

/// Check if a time (hour, minute, second) falls in the late start grace window:
/// `[09:00:00, 09:00:00 + grace_minutes]`
pub fn is_within_grace_window(hour: u8, minute: u8, second: u8, grace_minutes: u8) -> bool {
    let current_secs = (hour as u32) * 3600 + (minute as u32) * 60 + (second as u32);
    let target_secs =
        (CEREMONY_HOUR as u32) * 3600 + (CEREMONY_MINUTE as u32) * 60 + (CEREMONY_SECOND as u32);

    if current_secs < target_secs {
        return false;
    }

    let elapsed_secs = current_secs - target_secs;
    elapsed_secs <= (grace_minutes as u32) * 60
}

/// Evaluate whether the ceremony should trigger.
///
/// Handles weekdays-only check, compensation window, and late-start grace window.
pub fn should_trigger_ceremony(
    hour: u8,
    minute: u8,
    second: u8,
    weekday_mon_to_sun: u8, // 1 = Monday, 7 = Sunday
    settings: &Settings,
    already_activated_today: bool,
    voice_durations: &VoiceDurations,
) -> bool {
    if !settings.ceremony_enabled {
        return false;
    }

    if settings.weekdays_only && weekday_mon_to_sun > 5 {
        return false;
    }

    if already_activated_today {
        return false;
    }

    let compensation = get_settings_compensation(settings, voice_durations);

    let in_compensation = is_in_compensation_window(hour, minute, second, compensation);
    let in_grace = is_within_grace_window(hour, minute, second, settings.late_start_grace_minutes);

    in_compensation || in_grace
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnnouncementVoice, AudioPreset};

    const ANNOUNCEMENT: Duration = Duration::from_secs(40);
    const BELL: Duration = Duration::from_secs(9);

    fn compensation(preset: AudioPreset) -> Duration {
        get_compensation_duration(preset.blocks(), ANNOUNCEMENT, BELL)
    }

    #[test]
    fn presets_that_open_with_the_announcement_start_early_by_its_length() {
        for preset in [AudioPreset::VoiceSilenceBell, AudioPreset::VoiceSilence] {
            assert_eq!(compensation(preset), ANNOUNCEMENT, "{preset:?}");
        }
    }

    #[test]
    fn presets_that_open_with_a_bell_start_early_by_its_length() {
        assert_eq!(compensation(AudioPreset::BellSilenceBell), BELL);
    }

    #[test]
    fn the_pause_before_the_metronome_starts_the_ceremony_one_second_earlier() {
        for preset in [
            AudioPreset::VoiceMetronome,
            AudioPreset::VoiceMetronomeAnthem,
            AudioPreset::VoiceMetronomeEnding,
        ] {
            assert_eq!(
                compensation(preset),
                ANNOUNCEMENT + METRONOME_LEAD_PAUSE,
                "{preset:?}"
            );
        }
        assert_eq!(
            compensation(AudioPreset::BellMetronomeBell),
            BELL + METRONOME_LEAD_PAUSE
        );
    }

    #[test]
    fn presets_without_an_opening_sound_do_not_start_early() {
        for preset in [
            AudioPreset::MetronomeOnly,
            AudioPreset::MetronomeAnthem,
            AudioPreset::Silence,
        ] {
            assert_eq!(compensation(preset), Duration::ZERO, "{preset:?}");
        }
    }

    #[test]
    fn a_bell_built_into_the_announcement_is_not_counted_twice() {
        let blocks = effective_blocks(
            &[
                PresetBlock::Bell,
                PresetBlock::Announcement,
                PresetBlock::Metronome,
            ],
            AnnouncementVoice::VinnytsiaCity,
        );
        assert_eq!(
            get_compensation_duration(&blocks, ANNOUNCEMENT, BELL),
            ANNOUNCEMENT + METRONOME_LEAD_PAUSE
        );
    }

    #[test]
    fn a_bell_and_an_announcement_both_count_towards_the_lead_in() {
        let blocks = [
            PresetBlock::Bell,
            PresetBlock::Announcement,
            PresetBlock::Metronome,
            PresetBlock::Bell,
        ];
        assert_eq!(
            get_compensation_duration(&blocks, ANNOUNCEMENT, BELL),
            ANNOUNCEMENT + BELL + METRONOME_LEAD_PAUSE
        );
    }

    #[test]
    fn test_compensation_window_exact_boundaries() {
        let comp = Duration::from_secs(31); // 09:00:00 - 31s = 08:59:29
        assert!(!is_in_compensation_window(8, 59, 28, comp));
        assert!(is_in_compensation_window(8, 59, 29, comp));
        assert!(is_in_compensation_window(8, 59, 59, comp));
        assert!(!is_in_compensation_window(9, 0, 0, comp));
    }

    #[test]
    fn test_grace_window_exact_boundaries() {
        let grace = 3; // up to 09:03:00
        assert!(!is_within_grace_window(8, 59, 59, grace));
        assert!(is_within_grace_window(9, 0, 0, grace));
        assert!(is_within_grace_window(9, 3, 0, grace));
        assert!(!is_within_grace_window(9, 3, 1, grace));
    }

    #[test]
    fn test_should_trigger_ceremony_scenarios() {
        let mut settings = Settings::default();
        settings.ceremony_enabled = true;
        settings.weekdays_only = true;
        settings.late_start_grace_minutes = 2;

        let durations = VoiceDurations {
            announcement: Duration::from_secs(30),
            bell: Duration::from_secs(4),
            ending: None,
        };

        // Sunday (weekday 7) with weekdays_only -> false
        assert!(!should_trigger_ceremony(
            8, 59, 30, 7, &settings, false, &durations
        ));

        // Monday (weekday 1) during compensation -> true
        assert!(should_trigger_ceremony(
            8, 59, 30, 1, &settings, false, &durations
        ));

        // Already activated today -> false
        assert!(!should_trigger_ceremony(
            8, 59, 30, 1, &settings, true, &durations
        ));

        // In grace window -> true
        assert!(should_trigger_ceremony(
            9, 1, 0, 1, &settings, false, &durations
        ));

        // Outside grace window -> false
        assert!(!should_trigger_ceremony(
            9, 5, 0, 1, &settings, false, &durations
        ));
    }
}
