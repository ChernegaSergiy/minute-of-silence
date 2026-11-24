//! Starts the shared Rust scheduler from the Android foreground service.

use super::device;
use crate::Settings;
use crate::app::notification_actions::NotificationActionRequest;
use crate::core::settings::{PersonalDate, read_store_personal_dates, read_store_settings};
use crate::state::AppState;
use crate::state::CeremonyRuntime;
use chrono::{DateTime, Local};
use jni::JNIEnv;
use jni::objects::{JObject, JString};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Manager;

static RUNTIME: OnceLock<Arc<CeremonyRuntime>> = OnceLock::new();
static SCHEDULER_RUNNING: AtomicBool = AtomicBool::new(false);
static SETTINGS_FILE_LOCK: Mutex<()> = Mutex::new(());

struct SchedulerRunningGuard;

impl Drop for SchedulerRunningGuard {
    fn drop(&mut self) {
        SCHEDULER_RUNNING.store(false, Ordering::Release);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NotificationActionResult {
    ceremony_enabled: bool,
    weekdays_only: bool,
    skip_date: Option<String>,
    app_state_handled: bool,
}

fn start_marker() -> Option<PathBuf> {
    device::data_dir().map(|dir| dir.join("last-scheduled-ceremony"))
}

/// Uses a single state for the service, the Test command, and later UI attachment.
pub(crate) fn runtime(settings: Settings) -> Arc<CeremonyRuntime> {
    Arc::clone(RUNTIME.get_or_init(|| {
        let runtime = CeremonyRuntime::new(settings);
        runtime.lock().last_activation = start_marker()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|raw| DateTime::parse_from_rfc3339(&raw).ok())
            .map(|started| started.with_timezone(&Local));
        Arc::new(runtime)
    }))
}

/// Keeps a scheduled run from starting again if the process is restarted.
pub(crate) fn remember_start(started: DateTime<Local>) {
    if let Some(path) = start_marker() {
        let temporary = path.with_extension("tmp");
        if let Err(error) = std::fs::write(&temporary, started.to_rfc3339())
            .and_then(|()| std::fs::rename(temporary, path))
        {
            log::warn!("Failed to save the scheduled ceremony start: {error}");
        }
    }
}

#[unsafe(no_mangle)]
extern "system" fn Java_ua_pp_khvylyna_MinuteOfSilence_CeremonyDevice_startSchedulerNative(
    _env: JNIEnv<'_>,
    _host: JObject<'_>,
) {
    if SCHEDULER_RUNNING.swap(true, Ordering::AcqRel) {
        return;
    }
    let running_guard = SchedulerRunningGuard;
    tauri::async_runtime::spawn(async move {
        let _running_guard = running_guard;
        if let Some(runtime) = load_runtime() {
            crate::core::scheduler::run(runtime).await;
        }
    });
}

fn load_runtime() -> Option<Arc<CeremonyRuntime>> {
    if let Some(runtime) = RUNTIME.get() {
        return Some(Arc::clone(runtime));
    }
    if let Some(language) = device::language() {
        rust_i18n::set_locale(language);
    }
    let directory = device::data_dir()?;
    match read_store_settings(&directory.join("settings.json")) {
        Ok(settings) => Some(runtime(settings)),
        Err(error) => {
            log::error!("Failed to load Android ceremony settings: {error}");
            None
        }
    }
}

/// Loads personal dates from the Tauri store while the Android service has no Activity.
pub(crate) fn read_personal_dates() -> crate::error::Result<Vec<PersonalDate>> {
    let directory = device::data_dir()
        .ok_or_else(|| crate::AppError::Platform("Android data directory is unavailable".into()))?;
    let _guard = SETTINGS_FILE_LOCK.lock().map_err(|_| {
        crate::AppError::Platform("Android settings file lock is unavailable".into())
    })?;
    read_store_personal_dates(&directory.join("settings.json"))
}

/// Clears a consumed skip date from the Android settings store and runtime.
pub(crate) fn clear_skip_date(runtime: &Arc<CeremonyRuntime>) -> crate::error::Result<()> {
    if let Some(app) = runtime.app() {
        let state = app.try_state::<AppState>().ok_or_else(|| {
            crate::AppError::Platform("Android application state is unavailable".into())
        })?;
        if state.lock().settings.skip_date.is_some() {
            state.update_settings(|settings| settings.skip_date = None)?;
        }
        return Ok(());
    }

    let directory = device::data_dir()
        .ok_or_else(|| crate::AppError::Platform("Android data directory is unavailable".into()))?;
    let path = directory.join("settings.json");
    let _guard = SETTINGS_FILE_LOCK.lock().map_err(|_| {
        crate::AppError::Platform("Android settings file lock is unavailable".into())
    })?;
    let mut settings = read_store_settings(&path)?;
    if settings.skip_date.is_none() {
        return Ok(());
    }
    settings.skip_date = None;
    settings
        .validate()
        .map_err(|error| crate::AppError::Settings(error.to_string()))?;

    let mut document: Value = match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(error) => return Err(error.into()),
    };
    let object = document.as_object_mut().ok_or_else(|| {
        crate::AppError::Settings("Android settings store must be a JSON object".into())
    })?;
    object.insert("settings".into(), serde_json::to_value(&settings)?);
    let serialized = serde_json::to_vec_pretty(&document)?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serialized)?;
    std::fs::rename(&temporary, &path)?;
    runtime.lock().settings = settings;
    Ok(())
}

#[unsafe(no_mangle)]
extern "system" fn Java_ua_pp_khvylyna_MinuteOfSilence_CeremonyDevice_handleNotificationActionNative(
    mut env: JNIEnv<'_>,
    _host: JObject<'_>,
    action: JString<'_>,
    target_date: JString<'_>,
) -> jni::sys::jstring {
    let result = (|| -> std::result::Result<jni::sys::jstring, String> {
        let action: String = env
            .get_string(&action)
            .map_err(|error| error.to_string())?
            .into();
        let target_date: String = env
            .get_string(&target_date)
            .map_err(|error| error.to_string())?
            .into();
        let payload = serde_json::json!({ "action": action, "targetDate": target_date });
        let request: NotificationActionRequest =
            serde_json::from_value(payload).map_err(|error| error.to_string())?;
        let response = apply_notification_action(request).map_err(|error| error.to_string())?;
        env.new_string(response)
            .map(|value| value.into_raw())
            .map_err(|error| error.to_string())
    })();

    match result {
        Ok(response) => response,
        Err(error) => {
            let _ = env.throw_new("java/lang/IllegalStateException", error);
            std::ptr::null_mut()
        }
    }
}

fn apply_notification_action(request: NotificationActionRequest) -> crate::error::Result<String> {
    let directory = device::data_dir()
        .ok_or_else(|| crate::AppError::Platform("Android data directory is unavailable".into()))?;
    let path = directory.join("settings.json");
    let runtime = load_runtime().ok_or_else(|| {
        crate::AppError::Platform("Android ceremony runtime is unavailable".into())
    })?;

    let (settings, app_state_handled) = if let Some(app) = runtime.app() {
        let state = app.try_state::<AppState>().ok_or_else(|| {
            crate::AppError::Platform("Android application state is unavailable".into())
        })?;
        let settings = state.update_settings(|settings| {
            request.apply(settings, Local::now());
        })?;
        (settings, true)
    } else {
        let _guard = SETTINGS_FILE_LOCK.lock().map_err(|_| {
            crate::AppError::Platform("Android settings file lock is unavailable".into())
        })?;
        let mut settings = read_store_settings(&path)?;
        request.apply(&mut settings, Local::now());
        settings
            .validate()
            .map_err(|error| crate::AppError::Settings(error.to_string()))?;

        let mut document: Value = match std::fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
            Err(error) => return Err(error.into()),
        };
        let object = document.as_object_mut().ok_or_else(|| {
            crate::AppError::Settings("Android settings store must be a JSON object".into())
        })?;
        object.insert("settings".into(), serde_json::to_value(&settings)?);
        let serialized = serde_json::to_vec_pretty(&document)?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, serialized)?;
        std::fs::rename(&temporary, &path)?;
        runtime.lock().settings = settings.clone();
        (settings, false)
    };

    serde_json::to_string(&NotificationActionResult {
        ceremony_enabled: settings.ceremony_enabled,
        weekdays_only: settings.weekdays_only,
        skip_date: settings
            .skip_date
            .map(|date| date.format("%Y-%m-%d").to_string()),
        app_state_handled,
    })
    .map_err(Into::into)
}
