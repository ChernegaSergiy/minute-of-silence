//! Starts the shared Rust scheduler from the Android foreground service.

use super::device;
use crate::Settings;
use crate::core::settings::read_store_settings;
use crate::state::CeremonyRuntime;
use chrono::{DateTime, Local};
use jni::JNIEnv;
use jni::objects::JObject;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

static RUNTIME: OnceLock<Arc<CeremonyRuntime>> = OnceLock::new();
static SCHEDULER_RUNNING: AtomicBool = AtomicBool::new(false);

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
    tauri::async_runtime::spawn(async {
        if let Some(runtime) = load_runtime() {
            crate::core::scheduler::run(runtime).await;
        }
        SCHEDULER_RUNNING.store(false, Ordering::Release);
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
