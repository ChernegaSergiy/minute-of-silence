//! Persistent application settings and audio presets.
//!
//! Handles loading, saving, and providing default values for user
//! configurations such as volume, selected preset, and scheduling options.

use std::path::PathBuf;

pub use khvylyna_config::{
    AnnouncementVoice, AnthemVoice, AudioPreset, Settings, UiTheme, CONFIG_SCHEMA_VERSION,
};

use crate::error::{AppError, Result};

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
                let settings: Settings = serde_json::from_value(val).map_err(|e| e.to_string())?;
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
        let settings: Settings = serde_json::from_str(&raw)?;
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
