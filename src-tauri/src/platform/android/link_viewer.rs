use crate::error::{AppError, Result};
use std::sync::OnceLock;
use tauri::Wry;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};

const PLUGIN_IDENTIFIER: &str = "ua.pp.khvylyna.MinuteOfSilence";
const PLUGIN_CLASS: &str = "LinkViewerPlugin";

static HANDLE: OnceLock<PluginHandle<Wry>> = OnceLock::new();

/// Registers the Android link viewer plugin.
pub fn init() -> TauriPlugin<Wry> {
    Builder::<Wry>::new("link-viewer")
        .setup(|_app, api| {
            let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, PLUGIN_CLASS)?;
            if HANDLE.set(handle).is_err() {
                log::warn!("The Android link viewer plugin was registered twice");
            }
            Ok(())
        })
        .build()
}

/// Opens a secure URL in the Android in-app WebView.
pub async fn open(url: &str) -> Result<()> {
    let handle = HANDLE.get().ok_or_else(|| {
        AppError::Platform("The Android link viewer plugin is not registered yet".to_string())
    })?;

    handle
        .run_mobile_plugin_async::<()>("openLink", serde_json::json!({ "url": url }))
        .await
        .map_err(|error| AppError::Platform(format!("Failed to open the link viewer: {error}")))
}
