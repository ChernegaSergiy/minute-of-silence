//! Runtime permissions the ceremony needs on Android.

use super::bridge;
use crate::error::Result;
use crate::platform::NotificationPermission;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct PermissionResponse {
    granted: bool,
}

/// Asks the user to allow notifications, which Android 13 and newer require.
///
/// Waits for the answer to the system dialog and returns whether the
/// permission is granted. It returns at once when the permission is already
/// granted or the Android version does not ask for it.
pub async fn request_notifications() -> Result<bool> {
    let response: PermissionResponse =
        bridge::call_async("requestNotificationPermission", json!({})).await?;
    Ok(response.granted)
}

#[derive(Deserialize)]
struct StateResponse {
    state: NotificationPermission,
}

/// Returns the state of the notification permission without asking.
pub async fn status() -> Result<NotificationPermission> {
    let response: StateResponse =
        bridge::call_async("getNotificationPermission", json!({})).await?;
    Ok(response.state)
}

/// Opens the system notification settings of the app.
pub async fn open_settings() -> Result<()> {
    bridge::call_async("openNotificationSettings", json!({})).await
}
