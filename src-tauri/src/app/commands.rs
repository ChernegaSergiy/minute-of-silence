//! Tauri IPC commands exposed to the frontend via `invoke()`.

use std::{
    collections::VecDeque,
    fs,
    io::{BufRead, BufReader},
};

use tauri::{AppHandle, Manager, State};

use crate::{
    Result,
    app::next_skip_date,
    state::{AppState, StatusSnapshot},
};

// Settings

/// Retrieve current application settings from runtime state.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> crate::core::settings::Settings {
    let inner = state.lock();
    inner.settings.clone()
}

/// Retrieve complete initial application state for atomic startup hydration.
#[tauri::command]
pub fn get_initial_state(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<khvylyna_core::AppInitialState> {
    let settings = state.lock().settings.clone();
    let personal_dates = get_personal_dates(app.clone())?;
    let status = state.get_snapshot();
    let version = app.package_info().version.to_string();

    Ok(khvylyna_core::AppInitialState {
        settings,
        personal_dates,
        status,
        version,
    })
}

/// Save application settings, validate, update runtime state, and persist to store.
#[tauri::command]
pub fn save_settings(
    _app: AppHandle,
    state: State<'_, AppState>,
    mut settings: crate::core::settings::Settings,
) -> Result<crate::core::settings::Settings> {
    settings
        .validate()
        .map_err(|e| crate::AppError::Settings(e.to_string()))?;

    let saved = state.update_settings(|current| {
        // Preserve device-local state managed independently by scheduler / tray / skip
        // commands and the ceremony overlay
        settings.skip_date = current.skip_date;
        settings.last_shown_nearby_date_id = current.last_shown_nearby_date_id.clone();
        *current = settings;
    })?;

    log::info!("Settings successfully saved");
    Ok(saved)
}

/// Reset application settings to default values, persist to store, and update runtime state.
#[tauri::command]
pub fn reset_settings(
    _app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::core::settings::Settings> {
    let defaults = crate::core::settings::Settings::default();
    let saved = state.update_settings(|s| {
        *s = defaults;
    })?;

    log::info!("Settings successfully reset to defaults");
    Ok(saved)
}

/// Export current application settings and personal dates as a JSON bundle.
///
/// Device-local state (skip date, last shown nearby date) is not exported.
#[tauri::command]
pub fn export_settings(app: AppHandle, state: State<'_, AppState>) -> Result<String> {
    let settings = {
        let inner = state.lock();
        inner.settings.clone()
    };
    let personal_dates = get_personal_dates(app)?;
    let bundle = khvylyna_core::ExportBundle {
        settings,
        personal_dates,
    };
    khvylyna_core::export_bundle_to_json(&bundle)
        .map_err(|e| crate::AppError::Settings(e.to_string()))
}

/// Import an export bundle from a JSON string, validate, and persist settings and personal dates together.
///
/// A legacy settings-only file carries no personal dates, so the saved ones are kept.
#[tauri::command]
pub fn import_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    json: String,
) -> Result<khvylyna_core::ExportBundle> {
    let imported = khvylyna_core::import_bundle_from_json(&json)
        .map_err(|e| crate::AppError::Settings(format!("Invalid settings format: {e}")))?;

    // Settings and personal dates (when the file contains them) are persisted together.
    let mut imported_settings = imported.settings;
    let saved = state.update_settings_and_personal_dates(
        |current| {
            // Keep device-local state: skip_date is managed by scheduler / tray / skip
            // commands and the last shown nearby date by the overlay.
            imported_settings.skip_date = current.skip_date;
            imported_settings.last_shown_nearby_date_id = current.last_shown_nearby_date_id.clone();
            *current = imported_settings.clone();
        },
        imported.personal_dates.as_deref(),
    )?;
    let personal_dates = match imported.personal_dates {
        Some(dates) => dates,
        None => get_personal_dates(app)?,
    };

    log::info!(
        "Settings imported (version: {}, personal_dates: {})",
        saved.version,
        personal_dates.len()
    );
    Ok(khvylyna_core::ExportBundle {
        settings: saved,
        personal_dates,
    })
}

/// Sync the persisted autostart setting with the actual platform state.
#[tauri::command]
pub fn sync_autostart_from_system(state: State<'_, AppState>) -> Result<()> {
    crate::platform::sync_autostart_from_system(state)
}

// Personal Dates

/// Retrieve saved personal remembrance dates from store.
#[tauri::command]
pub fn get_personal_dates(app: AppHandle) -> Result<Vec<crate::core::settings::PersonalDate>> {
    use tauri_plugin_store::StoreExt;
    let store = app
        .store("settings.json")
        .map_err(|e| crate::AppError::Settings(e.to_string()))?;
    if let Some(val) = store.get("personal_dates") {
        let dates: Vec<crate::core::settings::PersonalDate> =
            serde_json::from_value(val).map_err(|e| crate::AppError::Settings(e.to_string()))?;
        Ok(dates)
    } else {
        Ok(Vec::new())
    }
}

/// Save personal remembrance dates after validation.
#[tauri::command]
pub fn save_personal_dates(
    state: State<'_, AppState>,
    dates: Vec<crate::core::settings::PersonalDate>,
) -> Result<()> {
    for d in &dates {
        d.validate()
            .map_err(|e| crate::AppError::Settings(e.to_string()))?;
    }
    state.save_personal_dates(&dates)
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
pub fn skip_next(_app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let skip_date = next_skip_date(chrono::Local::now());
    state.update_settings(|s| {
        s.skip_date = Some(skip_date);
    })?;
    log::info!("Next ceremony skipped (date: {skip_date})");
    Ok(())
}

/// Remove the skip flag for the next calendar day.
#[tauri::command]
pub fn unskip_next(_app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.update_settings(|s| {
        s.skip_date = None;
    })?;
    log::info!("Skip for next ceremony removed");
    Ok(())
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
    crate::core::scheduler::trigger_now(app, true).await;
    Ok(())
}

/// Finish the ceremony early (called by frontend when audio playback completes).
#[tauri::command]
pub async fn finish_ceremony_now(app: AppHandle) -> Result<()> {
    log::info!("Ceremony finish requested by audio engine");
    let platform = crate::platform::get_platform();
    crate::core::CeremonyManager::finish_ceremony(app, platform).await;
    Ok(())
}

/// Return debug info and a tail of the latest application log file.
#[tauri::command]
pub fn get_log_contents(app: AppHandle) -> Result<String> {
    const MAX_LOG_TAIL_LINES: usize = 200;

    let log_dir = app.path().app_log_dir()?;
    let package = app.package_info();
    let log_path = log_dir.join(format!("{}.log", package.name));

    let mut lines = vec![
        format!("app: {}", package.name),
        format!("version: {}", package.version),
        format!("os: {}", std::env::consts::OS),
        format!("arch: {}", std::env::consts::ARCH),
        format!("log_dir: {}", log_dir.display()),
    ];

    if log_path.is_file() {
        let file = fs::File::open(&log_path)?;
        let reader = BufReader::new(file);
        let mut tail = VecDeque::with_capacity(MAX_LOG_TAIL_LINES);

        for line in reader.lines() {
            let line = line?;
            if tail.len() == MAX_LOG_TAIL_LINES {
                tail.pop_front();
            }
            tail.push_back(line);
        }

        lines.push(format!("log_file: {}", log_path.display()));
        lines.push(format!("lines_copied: {}", tail.len()));
        lines.push(String::from("---"));
        lines.extend(tail);
    } else {
        lines.push(String::from("log_file: not found"));
        lines.push(String::from("lines_copied: 0"));
        lines.push(String::from("---"));
    }

    Ok(lines.join("\n"))
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub date: Option<String>,
    pub body: Option<String>,
}

/// Check for updates and store the result in AppState.
#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<UpdateInfo>> {
    if !crate::platform::should_check_for_updates() {
        return Ok(None);
    }

    // Check if there is already a pending update stored in state
    {
        let inner = state.lock();
        if let Some(ref update) = inner.pending_update {
            return Ok(Some(UpdateInfo {
                version: update.version.clone(),
                current_version: app.package_info().version.to_string(),
                date: update.date.map(|d| d.to_string()),
                body: update.body.clone(),
            }));
        }
    }

    use tauri_plugin_updater::UpdaterExt;
    log::info!("Checking for updates...");
    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| crate::AppError::Update(e.to_string()))?;
    match updater.check().await {
        Ok(Some(update)) => {
            log::info!("Update found: {:?}", update.version);
            let info = UpdateInfo {
                version: update.version.clone(),
                current_version: app.package_info().version.to_string(),
                date: update.date.map(|d| d.to_string()),
                body: update.body.clone(),
            };

            let mut inner = state.lock();
            inner.pending_update = Some(update);
            Ok(Some(info))
        }
        Ok(None) => {
            log::info!("No updates available.");
            Ok(None)
        }
        Err(e) => {
            log::error!("Failed to check for updates: {}", e);
            Err(crate::AppError::Update(e.to_string()))
        }
    }
}

/// Download and install the pending update.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    log::info!("Installing update...");
    let update = {
        let mut inner = state.lock();
        inner.pending_update.take()
    };

    let Some(update) = update else {
        return Err(crate::AppError::Update(
            "No pending update found".to_string(),
        ));
    };

    use tauri::Emitter;

    #[derive(serde::Serialize, Clone)]
    struct ProgressPayload {
        progress: f64,
        status: String,
    }

    let app_clone = app.clone();
    let mut downloaded = 0;

    let res = update
        .download_and_install(
            move |chunk_length, total_length| {
                downloaded += chunk_length;
                if let Some(total) = total_length {
                    let progress = (downloaded as f64 / total as f64) * 100.0;
                    let _ = app_clone.emit(
                        "update-progress",
                        ProgressPayload {
                            progress,
                            status: "downloading".to_string(),
                        },
                    );
                }
            },
            {
                let app = app.clone();
                move || {
                    let _ = app.emit(
                        "update-progress",
                        ProgressPayload {
                            progress: 100.0,
                            status: "installing".to_string(),
                        },
                    );
                }
            },
        )
        .await;

    match res {
        Ok(_) => {
            log::info!("Update installed.");
            let should_restart = {
                let mut inner = state.lock();
                if inner.ceremony_active {
                    inner.pending_restart = true;
                    false
                } else {
                    true
                }
            };
            if should_restart {
                log::info!("Restarting app...");
                app.restart();
            } else {
                log::info!("Ceremony is active. Deferring restart...");
            }
            Ok(())
        }
        Err(e) => {
            let err_str = e.to_string();
            log::error!("Failed to download and install update: {}", err_str);
            // Restore the update so the user can retry
            let mut inner = state.lock();
            if inner.pending_update.is_none() {
                inner.pending_update = Some(update);
            }
            Err(crate::AppError::Update(err_str))
        }
    }
}
