//! Runtime permissions the ceremony needs on Android.

use super::bridge;
use crate::error::Result;
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
