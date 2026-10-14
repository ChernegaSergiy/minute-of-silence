//! Shared configuration models, serialization, and validation for Minute of Silence.
//!
//! Designed to support both desktop (Tauri) and embedded environments (`no_std`).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use serde::{Deserialize, Serialize};

/// Current configuration schema version.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    CONFIG_SCHEMA_VERSION
}

fn default_ntp_server() -> String {
    "pool.ntp.org".to_string()
}

/// User-configurable application settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Schema version for configuration export and migrations.
    #[serde(default = "default_schema_version")]
    pub version: u32,

    /// Enable daily activation at 09:00.
    pub ceremony_enabled: bool,

    /// Enable app autostart when the system boots.
    pub autostart_enabled: bool,

    /// Run ceremony only on weekdays (Mon-Fri).
    pub weekdays_only: bool,

    /// Selected audio preset (1–10).
    pub preset: AudioPreset,

    /// Master volume for the ceremony audio (0–100).
    pub volume: u8,

    /// Pause other media players before the ceremony.
    pub pause_other_players: bool,

    /// Automatically resume paused players after the ceremony.
    pub resume_after_ceremony: bool,

    /// Show a visual overlay window when the ceremony starts.
    pub show_visual_overlay: bool,

    /// Show the flag animation window when the ceremony starts.
    pub show_flag_animation: bool,

    /// Show personal dates on nearby days with weighted proximity.
    pub show_nearby_personal_dates: bool,

    /// Use system time instead of NTP.
    pub system_time_only: bool,

    /// Prioritize app volume over system controls.
    pub volume_priority: bool,

    /// Automatically unmute system if muted during ceremony.
    pub auto_unmute: bool,

    /// NTP server hostname (used when system_time_only is false).
    pub ntp_server: String,

    /// If the system wakes from sleep after 09:00, activate if within this
    /// many minutes of the scheduled time (0 = never activate late).
    pub late_start_grace_minutes: u8,

    /// Enable reminder notifications.
    pub reminder_enabled: bool,

    /// Show a system notification N minutes before the ceremony (0 = immediately).
    /// Valid range: 0–10.
    pub reminder_minutes_before: u8,

    /// Selected announcement voice/version.
    pub announcement_voice: AnnouncementVoice,

    /// Selected anthem voice/performance.
    pub anthem_voice: AnthemVoice,

    /// Follow the OS theme when true; otherwise use `ui_theme`.
    pub use_system_theme: bool,

    /// Manual UI theme when `use_system_theme` is false.
    pub ui_theme: UiTheme,

    /// ID of the last personal date shown via nearby-days algorithm.
    #[serde(default)]
    pub last_shown_nearby_date_id: Option<String>,

    /// Date to skip the next ceremony (one-time skip). Persisted to disk.
    #[serde(default)]
    pub skip_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementVoice {
    BohdanHdal,
    SoniaSotnyk,
    DaniaKhomutovskyi,
    RadioBg,
    AirAlert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnthemVoice {
    Default,
    MykhailoKhoma,
    OleksandrPonomarov,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioPreset {
    /// Voice announcement + metronome
    VoiceMetronome,
    /// Metronome only (no voice)
    MetronomeOnly,
    /// Voice announcement + 60 s silence + bell
    VoiceSilenceBell,
    /// Voice announcement + 60 s silence
    VoiceSilence,
    /// Voice announcement + metronome + national anthem
    VoiceMetronomeAnthem,
    /// Voice announcement + metronome + ending
    VoiceMetronomeEnding,
    /// Metronome + national anthem
    MetronomeAnthem,
    /// Bell + 60 s silence + bell
    BellSilenceBell,
    /// Bell + metronome + bell
    BellMetronomeBell,
    /// Silence only (no audio, just visual overlay)
    Silence,
}

impl AudioPreset {
    pub fn has_anthem(self) -> bool {
        matches!(self, Self::VoiceMetronomeAnthem | Self::MetronomeAnthem)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: CONFIG_SCHEMA_VERSION,
            ceremony_enabled: true,
            autostart_enabled: true,
            weekdays_only: false,
            preset: AudioPreset::VoiceMetronome,
            volume: 80,
            pause_other_players: true,
            resume_after_ceremony: false,
            show_visual_overlay: true,
            show_flag_animation: false,
            show_nearby_personal_dates: false,
            system_time_only: false,
            volume_priority: false,
            auto_unmute: false,
            ntp_server: default_ntp_server(),
            late_start_grace_minutes: 1,
            reminder_enabled: false,
            reminder_minutes_before: 5,
            announcement_voice: AnnouncementVoice::BohdanHdal,
            anthem_voice: AnthemVoice::Default,
            use_system_theme: true,
            ui_theme: UiTheme::Light,
            last_shown_nearby_date_id: None,
            skip_date: None,
        }
    }
}

/// Serialize settings to formatted JSON string.
#[cfg(feature = "std")]
pub fn export_to_json(settings: &Settings) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(settings)
}

/// Deserialize settings from a JSON string.
#[cfg(feature = "std")]
pub fn import_from_json(json_str: &str) -> Result<Settings, serde_json::Error> {
    serde_json::from_str(json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings_version() {
        let settings = Settings::default();
        assert_eq!(settings.version, CONFIG_SCHEMA_VERSION);
    }

    #[test]
    fn test_audio_preset_anthem() {
        assert!(AudioPreset::VoiceMetronomeAnthem.has_anthem());
        assert!(AudioPreset::MetronomeAnthem.has_anthem());
        assert!(!AudioPreset::VoiceMetronome.has_anthem());
        assert!(!AudioPreset::Silence.has_anthem());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_json_roundtrip() {
        let settings = Settings::default();
        let json = export_to_json(&settings).expect("serialization failed");
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"ceremonyEnabled\": true"));

        let deserialized = import_from_json(&json).expect("deserialization failed");
        assert_eq!(settings, deserialized);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_backward_compatibility_missing_version() {
        let json_without_version = r#"{
            "ceremonyEnabled": false,
            "volume": 50
        }"#;

        let parsed: Settings = import_from_json(json_without_version).expect("should parse");
        assert_eq!(parsed.version, CONFIG_SCHEMA_VERSION);
        assert!(!parsed.ceremony_enabled);
        assert_eq!(parsed.volume, 50);
    }
}
