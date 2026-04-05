//! Tauri IPC commands exposed to the frontend via `invoke()`.

use tauri::{AppHandle, State};

#[allow(unused_imports)]
use crate::{
    core::settings::Settings,
    state::{AppState, StatusSnapshot},
    AppError, Result,
};

// Settings

/// Return the current settings snapshot.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.lock().settings.clone()
}

/// Persist updated settings and apply side-effects (e.g. autostart toggle).
#[tauri::command]
#[allow(unused_variables)]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<()> {
    // Persist to disk.
    settings.save()?;

    // Apply autostart setting via the plugin.
    #[cfg(not(test))]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autostart_manager = app.autolaunch();
        if settings.autostart_enabled {
            let _ = autostart_manager.enable();
        } else {
            let _ = autostart_manager.disable();
        }
    }

    // Update in-memory state.
    state.lock().settings = settings.clone();

    // Trigger immediate NTP sync if system time is disabled.
    if !settings.system_time_only {
        let ntp = state.ntp_service.clone();
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = ntp.sync().await;
            use tauri::Emitter;
            let _ = app_handle.emit("ntp-synced", ());
        });
    }

    log::info!("Settings saved");
    Ok(())
}

// Status

/// Return a lightweight runtime status snapshot.
#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> StatusSnapshot {
    state.get_snapshot()
}

// Skip / unskip

/// Skip the ceremony for the next calendar day.
#[tauri::command]
pub fn skip_next(state: State<'_, AppState>) {
    let tomorrow = (chrono::Local::now() + chrono::Duration::days(1)).date_naive();
    state.lock().skip_date = Some(tomorrow);
    log::info!("Next ceremony skipped (date: {tomorrow})");
}

/// Remove the skip flag for the next calendar day.
#[tauri::command]
pub fn unskip_next(state: State<'_, AppState>) {
    state.lock().skip_date = None;
    log::info!("Skip for next ceremony removed");
}

// Manual trigger

/// Force immediate NTP synchronization.
#[tauri::command]
pub async fn sync_ntp_now(state: State<'_, AppState>) -> Result<StatusSnapshot> {
    log::info!("Manual NTP sync requested");
    let _ = state.ntp_service.sync().await;
    Ok(state.get_snapshot())
}

/// Immediately trigger the ceremony (for testing / demonstration purposes).
#[tauri::command]
pub async fn trigger_ceremony_now(app: AppHandle) -> Result<()> {
    log::info!("Manual ceremony trigger requested");
    tauri::async_runtime::spawn(async move {
        crate::core::scheduler::trigger_now(app).await;
    });
    Ok(())
}

/// Finish the ceremony early (called by frontend when audio playback completes).
#[tauri::command]
pub async fn finish_ceremony_now(app: AppHandle) -> Result<()> {
    log::info!("Ceremony finish requested by audio engine");
    let platform = crate::core::platform::get_platform();
    crate::core::CeremonyManager::finish_ceremony(app, platform).await;
    Ok(())
}

/// Temporary diagnostic command — paste into src-tauri/src/commands.rs
/// and add to invoke_handler in lib.rs as `commands::list_audio_devices`
/// Remove after diagnosing.

#[tauri::command]
pub fn list_audio_devices() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::PROPERTYKEY;
        use windows::Win32::Media::Audio::{
            eRender, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
        };
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, STGM_READ};

        let form_factor_pkey = PROPERTYKEY {
            fmtid: windows::core::GUID::from_u128(0x1da5d803_d492_4edd_8c23_e0c0ffee7f0e),
            pid: 0,
        };

        // PKEY_Device_FriendlyName: {A45C254E-DF1C-4EFD-8020-67D146A850E0}, pid=14
        let friendly_name_pkey = PROPERTYKEY {
            fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
            pid: 14,
        };

        unsafe {
            let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(
                &MMDeviceEnumerator,
                None,
                CLSCTX_INPROC_SERVER,
            ) else {
                return vec!["ERROR: CoCreateInstance failed".to_string()];
            };

            let Ok(collection) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else {
                return vec!["ERROR: EnumAudioEndpoints failed".to_string()];
            };

            let Ok(count) = collection.GetCount() else {
                return vec!["ERROR: GetCount failed".to_string()];
            };

            let mut results = Vec::new();

            for i in 0..count {
                let Ok(device) = collection.Item(i) else {
                    continue;
                };

                let id = device
                    .GetId()
                    .ok()
                    .and_then(|id| id.to_string().ok())
                    .unwrap_or_else(|| "<no id>".to_string());

                let props = device.OpenPropertyStore(STGM_READ).ok();

                let form_factor: u32 = props
                    .as_ref()
                    .and_then(|p| p.GetValue(&form_factor_pkey).ok())
                    .map(|v| v.Anonymous.Anonymous.Anonymous.ulVal)
                    .unwrap_or(999);

                let friendly_name: String = props
                    .as_ref()
                    .and_then(|p| p.GetValue(&friendly_name_pkey).ok())
                    .and_then(|v| {
                        // VT_LPWSTR = 31
                        let vt = v.Anonymous.Anonymous.vt.0;
                        if vt == 31 {
                            let ptr = v.Anonymous.Anonymous.Anonymous.pwszVal;
                            if !ptr.is_null() {
                                return ptr.to_string().ok();
                            }
                        }
                        None
                    })
                    .unwrap_or_else(|| "<no name>".to_string());

                results.push(format!(
                    "FormFactor={} | Name={} | ID={}",
                    form_factor, friendly_name, id
                ));
            }

            if results.is_empty() {
                results.push("No active render devices found".to_string());
            }

            results
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec!["Not supported on this platform".to_string()]
    }
}
