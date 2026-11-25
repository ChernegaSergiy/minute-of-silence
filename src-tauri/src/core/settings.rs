//! Persistent application settings and audio presets.
//!
//! Handles loading, saving, and providing default values for user
//! configurations such as volume, selected preset, and scheduling options.

use std::path::PathBuf;

pub use khvylyna_core::{
    AnnouncementVoice, AnthemVoice, AudioPreset, CONFIG_SCHEMA_VERSION, PersonalDate, PresetBlock,
    Settings, UiTheme,
};

use crate::error::{AppError, Result};

#[cfg(any(target_os = "android", test))]
fn decode_store_settings(raw: &str) -> Result<Settings> {
    #[derive(serde::Deserialize)]
    struct Store {
        settings: Option<Settings>,
    }

    let mut settings = serde_json::from_str::<Store>(raw)?
        .settings
        .unwrap_or_default();
    settings.migrate();
    Ok(settings)
}

#[cfg(any(target_os = "android", test))]
fn decode_store_personal_dates(raw: &str) -> Result<Vec<PersonalDate>> {
    #[derive(serde::Deserialize)]
    struct Store {
        personal_dates: Option<Vec<PersonalDate>>,
    }

    let store = serde_json::from_str::<Store>(raw)?;
    Ok(store.personal_dates.unwrap_or_default())
}

/// Reads the Tauri store for Android's service before any Activity exists.
#[cfg(target_os = "android")]
pub(crate) fn read_store_settings(path: &std::path::Path) -> Result<Settings> {
    match std::fs::read_to_string(path) {
        Ok(raw) => decode_store_settings(&raw),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(error) => Err(error.into()),
    }
}

/// Reads personal dates from the Tauri store for Android's service before any Activity exists.
#[cfg(target_os = "android")]
pub(crate) fn read_store_personal_dates(path: &std::path::Path) -> Result<Vec<PersonalDate>> {
    match std::fs::read_to_string(path) {
        Ok(raw) => decode_store_personal_dates(&raw),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

/// Extension trait providing disk and Tauri store persistence for `Settings`.
pub trait SettingsExt {
    /// Load settings from disk, falling back to defaults on any error.
    fn load_or_default() -> Self;

    /// Load settings from the Tauri store, falling back to defaults on any error.
    fn load_from_store(app_handle: &tauri::AppHandle) -> Self;

    /// Save settings to the Tauri store.
    fn save_to_store(&self, app_handle: &tauri::AppHandle) -> Result<()>;

    /// Load settings directly from the filesystem JSON file.
    fn load() -> Result<Self>
    where
        Self: Sized;

    /// Save settings directly to the filesystem JSON file.
    fn save(&self) -> Result<()>;

    /// Config file path.
    fn path() -> Result<PathBuf>;
}

impl SettingsExt for Settings {
    fn load_or_default() -> Self {
        Self::load().unwrap_or_else(|e| {
            log::warn!("Failed to load settings ({e}), using defaults");
            Self::default()
        })
    }

    fn load_from_store(app_handle: &tauri::AppHandle) -> Self {
        use tauri_plugin_store::StoreExt;

        let load_impl = || -> std::result::Result<Settings, String> {
            let store = app_handle
                .store("settings.json")
                .map_err(|e| e.to_string())?;
            if let Some(val) = store.get("settings") {
                let mut settings: Settings =
                    serde_json::from_value(val).map_err(|e| e.to_string())?;
                settings.migrate();
                Ok(settings)
            } else {
                Ok(Settings::default())
            }
        };

        load_impl().unwrap_or_else(|e| {
            log::warn!("Failed to load settings from store: {}, using defaults", e);
            Settings::default()
        })
    }

    fn save_to_store(&self, app_handle: &tauri::AppHandle) -> Result<()> {
        use tauri_plugin_store::StoreExt;

        let store = app_handle
            .store("settings.json")
            .map_err(|e| crate::error::AppError::Settings(e.to_string()))?;

        let val = serde_json::to_value(self)?;
        store.set("settings".to_string(), val);
        store
            .save()
            .map_err(|e| crate::error::AppError::Settings(e.to_string()))?;
        Ok(())
    }

    fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)?;
        let mut settings: Settings = serde_json::from_str(&raw)?;
        settings.migrate();
        Ok(settings)
    }

    fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    fn path() -> Result<PathBuf> {
        let mut dir = dirs::config_dir()
            .ok_or_else(|| AppError::Settings("Cannot locate config directory".into()))?;
        dir.push("minute-of-silence");
        dir.push("settings.json");
        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_settings_preserve_schedule_and_migrate_legacy_audio() {
        let raw = r#"{
            "settings": {
                "version": 2,
                "ceremonyEnabled": false,
                "weekdaysOnly": true,
                "skipDate": "2026-11-19",
                "preset": "voice_metronome"
            },
            "personal_dates": []
        }"#;
        let settings = decode_store_settings(raw).expect("stored settings");
        assert!(!settings.ceremony_enabled);
        assert!(settings.weekdays_only);
        assert_eq!(
            settings.skip_date,
            chrono::NaiveDate::from_ymd_opt(2026, 11, 19)
        );
        assert_eq!(settings.blocks, AudioPreset::VoiceMetronome.blocks());
    }

    #[test]
    fn background_settings_do_not_treat_invalid_data_as_defaults() {
        assert!(decode_store_settings(r#"{"settings":{"ceremonyEnabled":"no"}}"#).is_err());
    }

    #[test]
    fn background_personal_dates_read_the_store_key() {
        let dates = decode_store_personal_dates(
            r#"{"personal_dates":[{"month":11,"day":24,"year":2026,"label":"Remember"}]}"#,
        )
        .expect("stored personal dates");
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0].label, "Remember");
    }

    #[test]
    fn background_personal_dates_default_to_empty_when_missing() {
        assert!(
            decode_store_personal_dates(r#"{"settings":{}}"#)
                .expect("store without personal dates")
                .is_empty()
        );
    }

    #[test]
    fn background_personal_dates_reject_invalid_data() {
        assert!(decode_store_personal_dates(r#"{"personal_dates":"invalid"}"#).is_err());
    }
}
