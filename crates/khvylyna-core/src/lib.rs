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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "lowercase")]
pub enum UiTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementVoice {
    BohdanHdal,
    SoniaSotnyk,
    DaniaKhomutovskyi,
    RadioBg,
    AirAlert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "snake_case")]
pub enum AnthemVoice {
    Default,
    MykhailoKhoma,
    OleksandrPonomarov,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
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

/// Serialisable runtime status snapshot for the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct StatusSnapshot {
    pub ceremony_active: bool,
    pub skip_tomorrow: bool,
    pub last_activation: Option<String>,
    pub last_ntp_sync: Option<String>,
}

/// A personal remembrance date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct PersonalDate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub month: u8,
    pub day: u8,
    pub year: i32,
    pub label: String,
}

impl PersonalDate {
    /// Validates month (1-12), day (1-31), and non-empty label.
    pub fn validate(&self) -> core::result::Result<(), ConfigError> {
        if self.month < 1 || self.month > 12 {
            return Err(ConfigError::InvalidField {
                field: "month",
                message: "month must be between 1 and 12",
            });
        }
        let max_day = match self.month {
            2 => 29,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if self.day < 1 || self.day > max_day {
            return Err(ConfigError::InvalidField {
                field: "day",
                message: "day must be between 1 and 31",
            });
        }
        if self.label.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: "label",
                message: "label cannot be empty",
            });
        }
        Ok(())
    }
}

/// Combined initial application state for atomic startup hydration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct AppInitialState {
    pub settings: Settings,
    pub personal_dates: Vec<PersonalDate>,
    pub status: StatusSnapshot,
    pub version: String,
}

/// Configuration import and validation errors.
#[derive(Debug)]
pub enum ConfigError {
    /// JSON serialization or deserialization error.
    #[cfg(feature = "std")]
    Json(serde_json::Error),
    /// Schema version is newer than supported.
    UnsupportedVersion { found: u32, max_supported: u32 },
    /// Schema version is zero or invalid.
    InvalidVersion(u32),
    /// A configuration field value is outside its valid range.
    InvalidField {
        field: &'static str,
        message: &'static str,
    },
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "std")]
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::UnsupportedVersion {
                found,
                max_supported,
            } => {
                write!(
                    f,
                    "Unsupported configuration schema version: {found} (maximum supported is {max_supported})"
                )
            }
            Self::InvalidVersion(v) => write!(f, "Invalid schema version: {v} (must be >= 1)"),
            Self::InvalidField { field, message } => {
                write!(f, "Invalid field '{field}': {message}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl Settings {
    /// Validates that the settings schema version is supported.
    pub fn validate_version(&self) -> Result<(), ConfigError> {
        if self.version == 0 {
            return Err(ConfigError::InvalidVersion(0));
        }
        if self.version > CONFIG_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                found: self.version,
                max_supported: CONFIG_SCHEMA_VERSION,
            });
        }
        Ok(())
    }

    /// Validates that settings conform to the schema version and field bounds.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_version()?;

        if self.volume > 100 {
            return Err(ConfigError::InvalidField {
                field: "volume",
                message: "Volume must be between 0 and 100",
            });
        }

        if self.late_start_grace_minutes > 5 {
            return Err(ConfigError::InvalidField {
                field: "lateStartGraceMinutes",
                message: "Late start grace minutes must be between 0 and 5",
            });
        }

        if self.reminder_minutes_before > 10 {
            return Err(ConfigError::InvalidField {
                field: "reminderMinutesBefore",
                message: "Reminder minutes before must be between 0 and 10",
            });
        }

        if self.ntp_server.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: "ntpServer",
                message: "NTP server hostname cannot be empty",
            });
        }

        Ok(())
    }
}

/// A portable export bundle that carries both application settings and personal dates.
///
/// Produced by [`export_bundle_to_json`] and consumed by [`import_bundle_from_json`].
/// Old export files that contain only a [`Settings`] object are accepted as well
/// (backward-compatible import).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    /// Application settings.
    pub settings: Settings,
    /// Personal remembrance dates.
    pub personal_dates: alloc::vec::Vec<PersonalDate>,
}

/// The result of importing a settings file.
///
/// Unlike [`ExportBundle`], `personal_dates` is `None` when the input was a legacy
/// settings-only file, so callers can keep the dates they already have instead of
/// overwriting them with an empty list.
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedBundle {
    /// Application settings.
    pub settings: Settings,
    /// Personal remembrance dates, or `None` if the input did not contain any.
    pub personal_dates: Option<alloc::vec::Vec<PersonalDate>>,
}

/// Serialize an [`ExportBundle`] to a formatted JSON string.
#[cfg(feature = "std")]
pub fn export_bundle_to_json(bundle: &ExportBundle) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(bundle)
}

/// Deserialize an [`ImportedBundle`] from a JSON string, validating settings and every personal date.
///
/// For backward compatibility, a JSON object that contains settings fields directly
/// (the old [`Settings`]-only export format) is also accepted — in that case
/// `personal_dates` is `None`.
#[cfg(feature = "std")]
pub fn import_bundle_from_json(json_str: &str) -> Result<ImportedBundle, ConfigError> {
    let input: serde_json::Value = serde_json::from_str(json_str)?;

    // A root-level `settings` or `personalDates` key marks the input as a bundle,
    // so a malformed bundle is rejected instead of being read as legacy settings.
    if input.get("settings").is_some() || input.get("personalDates").is_some() {
        let bundle: ExportBundle = serde_json::from_value(input)?;
        bundle.settings.validate()?;
        for date in &bundle.personal_dates {
            date.validate()?;
        }
        return Ok(ImportedBundle {
            settings: bundle.settings,
            personal_dates: Some(bundle.personal_dates),
        });
    }

    // Legacy Settings-only format.
    let settings: Settings = serde_json::from_value(input)?;
    settings.validate()?;
    Ok(ImportedBundle {
        settings,
        personal_dates: None,
    })
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
        let bundle = ExportBundle {
            settings: Settings::default(),
            personal_dates: alloc::vec![PersonalDate {
                id: Some("abc123".into()),
                month: 5,
                day: 9,
                year: 1945,
                label: "Victory Day".into(),
            }],
        };
        let json = export_bundle_to_json(&bundle).expect("serialization failed");
        assert!(json.contains("\"settings\""));
        assert!(json.contains("\"personalDates\""));
        assert!(json.contains("\"version\": 1"));

        let deserialized = import_bundle_from_json(&json).expect("deserialization failed");
        assert_eq!(bundle.settings, deserialized.settings);
        assert_eq!(Some(bundle.personal_dates), deserialized.personal_dates);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_bundle_backward_compat_settings_only() {
        // Old export format: Settings object at top level, no personalDates wrapper.
        let settings = Settings::default();
        let legacy_json = serde_json::to_string_pretty(&settings).expect("serialize");
        let bundle = import_bundle_from_json(&legacy_json).expect("legacy format should import");
        assert_eq!(bundle.settings, settings);
        assert_eq!(bundle.personal_dates, None);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_malformed_bundle_is_not_treated_as_legacy() {
        // `personalDates` has the wrong type; it must not fall back to default settings.
        let json = r#"{
            "settings": { "volume": 50 },
            "personalDates": "not-a-list"
        }"#;
        assert!(import_bundle_from_json(json).is_err());

        // Missing `personalDates` in a bundle is rejected as well.
        let json = r#"{ "settings": { "volume": 50 } }"#;
        assert!(import_bundle_from_json(json).is_err());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_backward_compatibility_missing_version() {
        // Bundle with a settings object that is missing the "version" field — should default to 1.
        let json_without_version = r#"{
            "settings": {
                "ceremonyEnabled": false,
                "volume": 50
            },
            "personalDates": []
        }"#;

        let bundle = import_bundle_from_json(json_without_version).expect("should parse");
        assert_eq!(bundle.settings.version, CONFIG_SCHEMA_VERSION);
        assert!(!bundle.settings.ceremony_enabled);
        assert_eq!(bundle.settings.volume, 50);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_reject_unsupported_future_version() {
        let json_future = r#"{
            "settings": {
                "version": 999,
                "ceremonyEnabled": true
            },
            "personalDates": []
        }"#;

        let err = import_bundle_from_json(json_future).unwrap_err();
        match err {
            ConfigError::UnsupportedVersion {
                found,
                max_supported,
            } => {
                assert_eq!(found, 999);
                assert_eq!(max_supported, CONFIG_SCHEMA_VERSION);
            }
            _ => panic!("Expected UnsupportedVersion, got {err:?}"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_reject_zero_version() {
        let json_zero = r#"{
            "settings": {
                "version": 0,
                "ceremonyEnabled": true
            },
            "personalDates": []
        }"#;

        let err = import_bundle_from_json(json_zero).unwrap_err();
        match err {
            ConfigError::InvalidVersion(v) => assert_eq!(v, 0),
            _ => panic!("Expected InvalidVersion, got {err:?}"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_reject_volume_above_100() {
        let json = r#"{
            "settings": { "volume": 101 },
            "personalDates": []
        }"#;
        let err = import_bundle_from_json(json).unwrap_err();
        match err {
            ConfigError::InvalidField { field, .. } => assert_eq!(field, "volume"),
            _ => panic!("Expected InvalidField for volume, got {err:?}"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_reject_grace_minutes_above_5() {
        let json = r#"{
            "settings": { "lateStartGraceMinutes": 6 },
            "personalDates": []
        }"#;
        let err = import_bundle_from_json(json).unwrap_err();
        match err {
            ConfigError::InvalidField { field, .. } => assert_eq!(field, "lateStartGraceMinutes"),
            _ => panic!("Expected InvalidField for lateStartGraceMinutes, got {err:?}"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_reject_reminder_minutes_above_10() {
        let json = r#"{
            "settings": { "reminderMinutesBefore": 11 },
            "personalDates": []
        }"#;
        let err = import_bundle_from_json(json).unwrap_err();
        match err {
            ConfigError::InvalidField { field, .. } => assert_eq!(field, "reminderMinutesBefore"),
            _ => panic!("Expected InvalidField for reminderMinutesBefore, got {err:?}"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_reject_empty_ntp_server() {
        let json = r#"{
            "settings": {
                "systemTimeOnly": true,
                "ntpServer": "   "
            },
            "personalDates": []
        }"#;
        let err = import_bundle_from_json(json).unwrap_err();
        match err {
            ConfigError::InvalidField { field, .. } => assert_eq!(field, "ntpServer"),
            _ => panic!("Expected InvalidField for ntpServer, got {err:?}"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_allow_valid_boundary_values() {
        let json = r#"{
            "settings": {
                "volume": 100,
                "lateStartGraceMinutes": 5,
                "reminderMinutesBefore": 10
            },
            "personalDates": []
        }"#;
        let bundle = import_bundle_from_json(json).expect("boundary values should be accepted");
        assert_eq!(bundle.settings.volume, 100);
        assert_eq!(bundle.settings.late_start_grace_minutes, 5);
        assert_eq!(bundle.settings.reminder_minutes_before, 10);
    }

    #[cfg(all(test, feature = "ts"))]
    #[test]
    fn test_export_ts() {
        use std::path::PathBuf;
        use ts_rs::TS;

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let src_dir = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root exists")
            .join("src");

        let cfg = ts_rs::Config::default().with_out_dir(src_dir);
        Settings::export_all(&cfg).expect("failed to export types");
        StatusSnapshot::export_all(&cfg).expect("failed to export StatusSnapshot");
        PersonalDate::export_all(&cfg).expect("failed to export PersonalDate");
        AppInitialState::export_all(&cfg).expect("failed to export AppInitialState");
        ExportBundle::export_all(&cfg).expect("failed to export ExportBundle");
    }

    #[test]
    fn test_personal_date_validation() {
        let valid = PersonalDate {
            id: Some("test-1".into()),
            month: 8,
            day: 24,
            year: 1991,
            label: "Independence Day".into(),
        };
        assert!(valid.validate().is_ok());

        let invalid_month = PersonalDate {
            month: 13,
            ..valid.clone()
        };
        assert!(invalid_month.validate().is_err());

        let invalid_day = PersonalDate {
            day: 0,
            ..valid.clone()
        };
        assert!(invalid_day.validate().is_err());

        let empty_label = PersonalDate {
            label: "   ".into(),
            ..valid
        };
        assert!(empty_label.validate().is_err());
    }
}
