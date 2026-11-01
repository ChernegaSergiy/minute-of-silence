//! Shared application state.

use chrono::{DateTime, Local};
pub use khvylyna_core::StatusSnapshot;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

use crate::app::next_skip_date;
use crate::core::audio::AudioEngine;
use crate::core::ntp_service::NtpService;
use crate::core::settings::{PersonalDate, Settings, SettingsExt};

use crate::error::Result;

rust_i18n::i18n!("locales");

use rust_i18n::t;

/// Runtime state shared between the scheduler, commands, and tray.
#[derive(Debug)]
pub struct AppState {
    inner: Arc<Mutex<Inner>>,
    settings_mutex: Mutex<()>,
    pub ntp_service: NtpService,
    pub audio: Arc<AudioEngine>,
    pub app_handle: AppHandle,
}

#[derive(Default)]
pub struct Inner {
    pub settings: Settings,
    pub ceremony_active: bool,
    pub last_activation: Option<DateTime<Local>>,
    pub pending_update: Option<tauri_plugin_updater::Update>,
    pub pending_restart: bool,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("settings", &self.settings)
            .field("ceremony_active", &self.ceremony_active)
            .field("last_activation", &self.last_activation)
            .field(
                "pending_update",
                &self.pending_update.as_ref().map(|u| &u.version),
            )
            .field("pending_restart", &self.pending_restart)
            .finish()
    }
}

/// Put a store key back to its previous in-memory value after a failed `save`.
fn restore_store_value(
    store: &tauri_plugin_store::Store<tauri::Wry>,
    key: &str,
    previous: Option<serde_json::Value>,
) {
    match previous {
        Some(val) => store.set(key.to_string(), val),
        None => {
            store.delete(key);
        }
    }
}

impl AppState {
    pub fn new(app_handle: AppHandle) -> Self {
        let settings = Settings::load_from_store(&app_handle);
        Self::new_with_settings(app_handle, settings)
    }

    pub fn new_with_settings(app_handle: AppHandle, settings: Settings) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                settings: settings.clone(),
                ceremony_active: false,
                last_activation: None,
                pending_update: None,
                pending_restart: false,
            })),
            settings_mutex: Mutex::new(()),
            ntp_service: NtpService::new(settings.ntp_server.clone()),
            audio: Arc::new(AudioEngine::new(app_handle.clone())),
            app_handle,
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("AppState mutex poisoned")
    }

    /// Atomically mutate settings with a serialized read-modify-persist-apply cycle.
    ///
    /// This serializes concurrent settings writes via a dedicated settings lock,
    /// preventing stale snapshot overwrites, while ensuring the runtime state lock
    /// is NOT held during disk I/O.
    pub fn update_settings<F>(&self, modify: F) -> Result<Settings>
    where
        F: FnOnce(&mut Settings),
    {
        let _guard = self.settings_mutex.lock().expect("settings_mutex poisoned");
        let old_settings = {
            let inner = self.lock();
            inner.settings.clone()
        };
        let mut settings = old_settings.clone();
        modify(&mut settings);
        if settings == old_settings {
            return Ok(settings);
        }
        settings
            .validate()
            .map_err(|e| crate::AppError::Settings(e.to_string()))?;
        settings.save_to_store(&self.app_handle)?;
        self.apply_settings_update(settings.clone());
        Ok(settings)
    }

    /// Atomically update settings and, optionally, personal dates with a single store write.
    ///
    /// Both values are staged in the store and flushed by one `save`, so a failed write
    /// never leaves settings and dates from different sources. On failure the in-memory
    /// store values are restored and the runtime state is left untouched.
    pub fn update_settings_and_personal_dates<F>(
        &self,
        modify: F,
        personal_dates: Option<&[PersonalDate]>,
    ) -> Result<Settings>
    where
        F: FnOnce(&mut Settings),
    {
        use tauri_plugin_store::StoreExt;

        let _guard = self.settings_mutex.lock().expect("settings_mutex poisoned");
        let mut settings = {
            let inner = self.lock();
            inner.settings.clone()
        };
        modify(&mut settings);
        settings
            .validate()
            .map_err(|e| crate::AppError::Settings(e.to_string()))?;

        // Serialize everything first so nothing is staged if serialization fails.
        let settings_val = serde_json::to_value(&settings)?;
        let dates_val = personal_dates.map(serde_json::to_value).transpose()?;

        let store = self
            .app_handle
            .store("settings.json")
            .map_err(|e| crate::AppError::Settings(e.to_string()))?;
        let previous_settings = store.get("settings");
        let previous_dates = store.get("personal_dates");

        store.set("settings".to_string(), settings_val);
        if let Some(val) = dates_val {
            store.set("personal_dates".to_string(), val);
        }
        if let Err(e) = store.save() {
            restore_store_value(&store, "settings", previous_settings);
            restore_store_value(&store, "personal_dates", previous_dates);
            return Err(crate::AppError::Settings(e.to_string()));
        }

        self.apply_settings_update(settings.clone());
        Ok(settings)
    }

    /// Persist personal dates under the same lock that serializes all settings writes.
    ///
    /// Without the lock a plain `set` + `save` could interleave with
    /// [`Self::update_settings_and_personal_dates`] and overwrite its dates. On a failed
    /// write the in-memory store value is restored.
    pub fn save_personal_dates(&self, personal_dates: &[PersonalDate]) -> Result<()> {
        use tauri_plugin_store::StoreExt;

        let _guard = self.settings_mutex.lock().expect("settings_mutex poisoned");
        let val = serde_json::to_value(personal_dates)?;
        let store = self
            .app_handle
            .store("settings.json")
            .map_err(|e| crate::AppError::Settings(e.to_string()))?;
        let previous = store.get("personal_dates");

        store.set("personal_dates".to_string(), val);
        if let Err(e) = store.save() {
            restore_store_value(&store, "personal_dates", previous);
            return Err(crate::AppError::Settings(e.to_string()));
        }
        Ok(())
    }

    /// Atomically update settings in runtime state and apply necessary side-effects.
    ///
    /// If settings have not changed, this is a no-op.
    pub fn apply_settings_update(&self, new_settings: Settings) {
        let (old_settings, changed) = {
            let mut inner = self.lock();
            let old = inner.settings.clone();
            if old == new_settings {
                (old, false)
            } else {
                inner.settings = new_settings.clone();
                (old, true)
            }
        };

        if !changed {
            return;
        }

        if old_settings.autostart_enabled != new_settings.autostart_enabled {
            log::info!(
                "Autostart setting changed: {} -> {}",
                old_settings.autostart_enabled,
                new_settings.autostart_enabled
            );
            crate::platform::apply_autostart_enabled(
                &self.app_handle,
                new_settings.autostart_enabled,
            );
        }

        let ntp_server_changed = old_settings.ntp_server != new_settings.ntp_server;
        if ntp_server_changed {
            log::info!(
                "NTP server changed: {} -> {}",
                old_settings.ntp_server,
                new_settings.ntp_server
            );
            self.ntp_service.set_server(new_settings.ntp_server.clone());
        }

        if ((old_settings.system_time_only != new_settings.system_time_only) || ntp_server_changed)
            && !new_settings.system_time_only
        {
            log::info!("NTP configuration changed, triggering immediate sync");
            let ntp = self.ntp_service.clone();
            let app_handle = self.app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let _ = ntp.sync().await;
                use tauri::Emitter;
                let _ = app_handle.emit("ntp-synced", ());
            });
        }
    }
}

impl AppState {
    pub fn get_snapshot(&self) -> StatusSnapshot {
        let inner = self.lock();
        let skip_target = next_skip_date(Local::now(), inner.settings.weekdays_only);

        let ntp_status = if inner.settings.system_time_only {
            Some(t!("ntp_disabled").to_string())
        } else {
            self.ntp_service
                .last_sync_time()
                .map(|dt| dt.format("%H:%M:%S").to_string())
                .or_else(|| Some(t!("ntp_syncing").to_string()))
        };

        StatusSnapshot {
            ceremony_active: inner.ceremony_active,
            skip_tomorrow: inner.settings.skip_date == Some(skip_target),
            last_activation: inner
                .last_activation
                .map(|dt| dt.format("%H:%M:%S").to_string()),
            last_ntp_sync: ntp_status,
        }
    }
}
