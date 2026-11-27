//! Shared configuration models, serialization, and validation for Minute of Silence.
//!
//! Designed to support both desktop (Tauri) and embedded environments (`no_std`).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod bundle;
pub mod error;
pub mod models;
pub mod scheduler;
pub mod voices;

// Flattened public API for backward compatibility and convenience.
pub use bundle::ExportBundle;
#[cfg(feature = "std")]
pub use bundle::{ImportedBundle, export_bundle_to_json, import_bundle_from_json};

pub use error::ConfigError;

pub use models::{
    AnnouncementVoice, AnthemVoice, AppInitialState, AudioPreset, CONFIG_SCHEMA_VERSION,
    DiscoveredDevice, PersonalDate, PresetBlock, Settings, StatusSnapshot, UiTheme,
    VoiceCapabilities, validate_blocks,
};

pub use scheduler::{
    CEREMONY_HOUR, CEREMONY_MINUTE, CEREMONY_SECOND, METRONOME_LEAD_PAUSE,
    get_compensation_duration, get_settings_compensation, is_in_compensation_window,
    is_within_grace_window, lead_in_pause, should_trigger_ceremony,
};

pub use voices::{VoiceAssets, VoiceDurations, effective_blocks, voice_capabilities};

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

    #[test]
    fn test_builtin_preset_blocks() {
        use PresetBlock::*;

        let expected: [(AudioPreset, &[PresetBlock]); 10] = [
            (AudioPreset::VoiceMetronome, &[Announcement, Metronome]),
            (AudioPreset::MetronomeOnly, &[Metronome]),
            (
                AudioPreset::VoiceSilenceBell,
                &[Announcement, Silence, Bell],
            ),
            (AudioPreset::VoiceSilence, &[Announcement, Silence]),
            (
                AudioPreset::VoiceMetronomeAnthem,
                &[Announcement, Metronome, Anthem],
            ),
            (
                AudioPreset::VoiceMetronomeEnding,
                &[Announcement, Metronome, Ending],
            ),
            (AudioPreset::MetronomeAnthem, &[Metronome, Anthem]),
            (AudioPreset::BellSilenceBell, &[Bell, Silence, Bell]),
            (AudioPreset::BellMetronomeBell, &[Bell, Metronome, Bell]),
            (AudioPreset::Silence, &[Silence]),
        ];

        for (preset, blocks) in expected {
            assert_eq!(preset.blocks(), blocks, "{preset:?}");
        }
    }

    #[test]
    fn test_has_anthem_matches_blocks() {
        for preset in [
            AudioPreset::VoiceMetronome,
            AudioPreset::MetronomeOnly,
            AudioPreset::VoiceSilenceBell,
            AudioPreset::VoiceSilence,
            AudioPreset::VoiceMetronomeAnthem,
            AudioPreset::VoiceMetronomeEnding,
            AudioPreset::MetronomeAnthem,
            AudioPreset::BellSilenceBell,
            AudioPreset::BellMetronomeBell,
            AudioPreset::Silence,
        ] {
            assert_eq!(
                preset.has_anthem(),
                preset.blocks().contains(&PresetBlock::Anthem),
                "{preset:?}"
            );
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_preset_block_serialization() {
        let json = serde_json::to_string(&PresetBlock::Silence).expect("serialization failed");
        assert_eq!(json, "\"silence\"");
        let block: PresetBlock = serde_json::from_str("\"announcement\"").expect("parse failed");
        assert_eq!(block, PresetBlock::Announcement);
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
        assert!(json.contains("\"version\": 2"));

        let deserialized = import_bundle_from_json(&json).expect("deserialization failed");
        assert_eq!(bundle.settings, deserialized.settings);
        assert_eq!(Some(bundle.personal_dates), deserialized.personal_dates);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_export_excludes_device_local_settings() {
        let bundle = ExportBundle {
            settings: Settings {
                last_shown_nearby_date_id: Some("abc123".into()),
                skip_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 15),
                ..Settings::default()
            },
            personal_dates: alloc::vec::Vec::new(),
        };
        let json = export_bundle_to_json(&bundle).expect("serialization failed");
        assert!(!json.contains("lastShownNearbyDateId"));
        assert!(!json.contains("skipDate"));
        assert!(json.contains("\"volume\""));

        // The excluded fields fall back to their defaults on import.
        let imported = import_bundle_from_json(&json).expect("import failed");
        assert_eq!(imported.settings.last_shown_nearby_date_id, None);
        assert_eq!(imported.settings.skip_date, None);
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
        // Bundle with a settings object that is missing the "version" field — should default to the current version.
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

    #[test]
    fn test_all_voices_are_listed_once() {
        for voice in AnnouncementVoice::ALL {
            // Adding a voice breaks this match, a reminder to add it to ALL too.
            match voice {
                AnnouncementVoice::BohdanHdal
                | AnnouncementVoice::SoniaSotnyk
                | AnnouncementVoice::DaniaKhomutovskyi
                | AnnouncementVoice::RadioBg
                | AnnouncementVoice::VinnytsiaCity => {}
            }
            let listed = AnnouncementVoice::ALL
                .iter()
                .filter(|other| **other == voice)
                .count();
            assert_eq!(listed, 1, "{voice:?}");
        }
    }

    #[test]
    fn test_only_silence_is_not_audible() {
        use PresetBlock::*;

        for block in [Announcement, Bell, Metronome, Ending, Anthem] {
            assert!(block.is_audible(), "{block:?}");
        }
        assert!(!Silence.is_audible());
    }

    #[test]
    fn test_default_blocks_match_default_preset() {
        let settings = Settings::default();
        assert_eq!(settings.blocks, settings.preset.blocks());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_builtin_presets_are_valid_block_layouts() {
        for preset in [
            AudioPreset::VoiceMetronome,
            AudioPreset::MetronomeOnly,
            AudioPreset::VoiceSilenceBell,
            AudioPreset::VoiceSilence,
            AudioPreset::VoiceMetronomeAnthem,
            AudioPreset::VoiceMetronomeEnding,
            AudioPreset::MetronomeAnthem,
            AudioPreset::BellSilenceBell,
            AudioPreset::BellMetronomeBell,
            AudioPreset::Silence,
        ] {
            assert_eq!(validate_blocks(preset.blocks()), Ok(()), "{preset:?}");
        }
    }

    #[test]
    fn test_validate_accepts_every_slot_combination() {
        use PresetBlock::*;

        for opening_bell in [false, true] {
            for announcement in [false, true] {
                for body in [Metronome, Silence] {
                    for closing in [None, Some(Ending), Some(Bell), Some(Anthem)] {
                        let mut blocks = alloc::vec::Vec::new();
                        if opening_bell {
                            blocks.push(Bell);
                        }
                        if announcement {
                            blocks.push(Announcement);
                        }
                        blocks.push(body);
                        blocks.extend(closing);
                        assert_eq!(validate_blocks(&blocks), Ok(()), "{blocks:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn test_validate_rejects_invalid_blocks() {
        use PresetBlock::*;

        let invalid: [&[PresetBlock]; 17] = [
            &[],
            &[Announcement, Announcement],
            &[Metronome, Silence],
            &[Bell, Metronome, Bell, Bell],
            &[Anthem, Metronome],
            &[Metronome, Anthem, Anthem],
            &[Ending, Ending],
            // No main part.
            &[Announcement],
            &[Bell],
            &[Anthem],
            &[Announcement, Ending],
            // Not in slot order.
            &[Ending, Metronome],
            &[Announcement, Bell, Metronome],
            &[Bell, Bell, Announcement, Metronome],
            // More than one closing block.
            &[Metronome, Ending, Anthem],
            &[Silence, Bell, Anthem],
            &[Metronome, Bell, Bell],
        ];
        for blocks in invalid {
            assert!(validate_blocks(blocks).is_err(), "{blocks:?}");
        }

        let settings = Settings {
            blocks: alloc::vec![],
            ..Settings::default()
        };
        assert!(matches!(
            settings.validate(),
            Err(ConfigError::InvalidField {
                field: "blocks",
                ..
            })
        ));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_migrate_fills_blocks_from_legacy_preset() {
        let legacy = r#"{ "version": 1, "preset": "bell_silence_bell", "volume": 50 }"#;
        let mut settings: Settings = serde_json::from_str(legacy).expect("parse failed");
        assert!(settings.blocks.is_empty());

        settings.migrate();

        assert_eq!(settings.version, CONFIG_SCHEMA_VERSION);
        assert_eq!(settings.volume, 50);
        assert_eq!(
            settings.blocks,
            [PresetBlock::Bell, PresetBlock::Silence, PresetBlock::Bell]
        );
        assert!(settings.validate().is_ok());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_migrate_turns_the_vinnytsia_ending_into_a_bell() {
        let legacy = r#"{ "version": 1, "preset": "voice_metronome_ending", "announcementVoice": "vinnytsia_city" }"#;
        let mut settings: Settings = serde_json::from_str(legacy).expect("parse failed");

        settings.migrate();

        assert_eq!(
            settings.blocks,
            [
                PresetBlock::Announcement,
                PresetBlock::Metronome,
                PresetBlock::Bell
            ]
        );
        assert!(settings.validate().is_ok());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_migrate_keeps_the_ending_of_other_voices() {
        let legacy = r#"{ "version": 1, "preset": "voice_metronome_ending", "announcementVoice": "sonia_sotnyk" }"#;
        let mut settings: Settings = serde_json::from_str(legacy).expect("parse failed");

        settings.migrate();

        assert_eq!(
            settings.blocks,
            [
                PresetBlock::Announcement,
                PresetBlock::Metronome,
                PresetBlock::Ending
            ]
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_unknown_voice_is_rejected() {
        let json = r#"{ "version": 2, "announcementVoice": "no_such_voice" }"#;

        assert!(serde_json::from_str::<Settings>(json).is_err());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_every_voice_survives_a_round_trip() {
        for voice in AnnouncementVoice::ALL {
            let json = serde_json::to_string(&voice).expect("serialize failed");
            let back: AnnouncementVoice = serde_json::from_str(&json).expect("parse failed");
            assert_eq!(back, voice, "{json}");
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_removed_air_alert_voice_reads_as_the_default_voice() {
        let json = r#"{ "version": 2, "announcementVoice": "air_alert", "volume": 50 }"#;
        let settings: Settings = serde_json::from_str(json).expect("parse failed");

        assert_eq!(settings.announcement_voice, AnnouncementVoice::BohdanHdal);
        assert_eq!(settings.volume, 50);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_known_voice_is_kept() {
        let json = r#"{ "version": 2, "announcementVoice": "vinnytsia_city" }"#;
        let settings: Settings = serde_json::from_str(json).expect("parse failed");

        assert_eq!(
            settings.announcement_voice,
            AnnouncementVoice::VinnytsiaCity
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_migrate_keeps_existing_blocks() {
        let current = r#"{ "version": 2, "preset": "silence", "blocks": ["metronome", "anthem"] }"#;
        let mut settings: Settings = serde_json::from_str(current).expect("parse failed");

        settings.migrate();

        assert_eq!(
            settings.blocks,
            [PresetBlock::Metronome, PresetBlock::Anthem]
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_migrate_keeps_invalid_versions() {
        for version in [0, CONFIG_SCHEMA_VERSION + 1] {
            let mut settings = Settings {
                version,
                ..Settings::default()
            };
            settings.migrate();
            assert_eq!(settings.version, version);
            assert!(settings.validate().is_err());
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_import_legacy_settings_migrates_blocks() {
        let legacy = r#"{ "version": 1, "preset": "metronome_anthem" }"#;
        let imported = import_bundle_from_json(legacy).expect("should import");

        assert_eq!(imported.settings.version, CONFIG_SCHEMA_VERSION);
        assert_eq!(
            imported.settings.blocks,
            [PresetBlock::Metronome, PresetBlock::Anthem]
        );
        assert!(imported.personal_dates.is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_import_bundle_migrates_blocks() {
        let legacy = r#"{
            "settings": { "version": 1, "preset": "voice_silence" },
            "personalDates": []
        }"#;
        let imported = import_bundle_from_json(legacy).expect("should import");

        assert_eq!(
            imported.settings.blocks,
            [PresetBlock::Announcement, PresetBlock::Silence]
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_import_rejects_invalid_blocks() {
        let json = r#"{ "version": 2, "blocks": ["anthem", "metronome"] }"#;
        assert!(matches!(
            import_bundle_from_json(json),
            Err(ConfigError::InvalidField {
                field: "blocks",
                ..
            })
        ));
    }
}
