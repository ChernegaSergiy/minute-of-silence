//! Shared application state.

use chrono::{DateTime, Local};
pub use khvylyna_core::StatusSnapshot;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

use crate::app::next_skip_date;
use crate::core::audio::AudioEngine;
use crate::core::ntp_service::NtpService;
use crate::core::settings::{Settings, SettingsExt};

rust_i18n::i18n!("locales");

use rust_i18n::t;

/// Runtime state shared between the scheduler, commands, and tray.
#[derive(Debug)]
pub struct AppState {
    inner: Arc<Mutex<Inner>>,
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
            ntp_service: NtpService::new(settings.ntp_server.clone()),
            audio: Arc::new(AudioEngine::new(app_handle.clone())),
            app_handle,
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("AppState mutex poisoned")
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
        let skip_target = next_skip_date(Local::now());

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
