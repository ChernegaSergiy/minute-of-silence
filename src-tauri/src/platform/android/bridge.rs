//! Bridge to the Kotlin `CeremonyPlugin` that runs inside the Android activity.
//!
//! The plugin class lives in `android-template` and is registered with Tauri's
//! mobile plugin system, so every call below is a plain JSON round trip.

use crate::error::{AppError, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::OnceLock;
use tauri::Wry;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};

/// Package of the Kotlin plugin class, equal to the Android application id.
const PLUGIN_IDENTIFIER: &str = "ua.pp.khvylyna.MinuteOfSilence";
/// Simple name of the Kotlin plugin class.
const PLUGIN_CLASS: &str = "CeremonyPlugin";

static HANDLE: OnceLock<PluginHandle<Wry>> = OnceLock::new();

/// Creates the Tauri plugin that loads the Kotlin `CeremonyPlugin`.
pub fn init() -> TauriPlugin<Wry> {
    Builder::<Wry>::new("ceremony")
        .setup(|_app, api| {
            let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, PLUGIN_CLASS)?;
            if HANDLE.set(handle).is_err() {
                log::warn!("The Android ceremony plugin was registered twice");
            }
            Ok(())
        })
        .build()
}

fn handle() -> Result<&'static PluginHandle<Wry>> {
    HANDLE.get().ok_or_else(|| {
        AppError::Platform("The Android ceremony plugin is not registered yet".to_string())
    })
}

fn failure(command: &str, error: impl std::fmt::Display) -> AppError {
    AppError::Platform(format!("Android command `{command}` failed: {error}"))
}

/// Runs a Kotlin command and blocks until it answers.
///
/// Never call it from the Android main thread.
pub fn call<T: DeserializeOwned>(command: &str, payload: impl Serialize) -> Result<T> {
    handle()?
        .run_mobile_plugin(command, payload)
        .map_err(|e| failure(command, e))
}

/// Runs a Kotlin command and awaits its answer.
pub async fn call_async<T: DeserializeOwned>(command: &str, payload: impl Serialize) -> Result<T> {
    handle()?
        .run_mobile_plugin_async(command, payload)
        .await
        .map_err(|e| failure(command, e))
}
