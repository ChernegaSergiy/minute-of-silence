//! Android ceremony runtime and the persisted scheduled start.

use super::device;
use crate::Settings;
use crate::state::CeremonyRuntime;
use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

static RUNTIME: OnceLock<Arc<CeremonyRuntime>> = OnceLock::new();

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
