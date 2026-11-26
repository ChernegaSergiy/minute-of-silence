//! Export and import bundles for settings and personal remembrance dates.

#[cfg(feature = "std")]
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
use crate::error::ConfigError;
use crate::models::dates::PersonalDate;
use crate::models::settings::Settings;

/// A portable export bundle that carries both application settings and personal dates.
///
/// Produced by [`export_bundle_to_json`] and consumed by [`import_bundle_from_json`].
/// Old export files that contain only a [`Settings`] object are accepted as well
/// (backward-compatible import).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    /// Application settings.
    pub settings: Settings,
    /// Personal remembrance dates.
    pub personal_dates: Vec<PersonalDate>,
}

/// The result of importing a settings file.
///
/// Unlike [`ExportBundle`], `personal_dates` is `None` when the input was a legacy
/// settings-only file, so callers can keep the dates they already have instead of
/// overwriting them with an empty list.
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedBundle {
    /// Application settings.
    pub settings: Settings,
    /// Personal remembrance dates, or `None` if the input did not contain any.
    pub personal_dates: Option<Vec<PersonalDate>>,
}

/// Settings fields that hold device-local state and are never written to an export.
#[cfg(feature = "std")]
const EXPORT_EXCLUDED_SETTINGS_FIELDS: [&str; 2] = ["lastShownNearbyDateId", "skipDate"];

/// Serialize an [`ExportBundle`] to a formatted JSON string.
///
/// Device-local settings (`lastShownNearbyDateId`, `skipDate`) are left out of the
/// output, so an export only carries what makes sense on another device.
#[cfg(feature = "std")]
pub fn export_bundle_to_json(bundle: &ExportBundle) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(bundle)?;
    if let Some(settings) = value
        .get_mut("settings")
        .and_then(serde_json::Value::as_object_mut)
    {
        for field in EXPORT_EXCLUDED_SETTINGS_FIELDS {
            settings.remove(field);
        }
    }
    serde_json::to_string_pretty(&value)
}

/// Deserialize an [`ImportedBundle`] from a JSON string, validating settings and every personal date.
///
/// For backward compatibility, a JSON object that contains settings fields directly
/// (the old [`Settings`]-only export format) is also accepted — in that case
/// `personal_dates` is `None`.
#[cfg(feature = "std")]
pub fn import_bundle_from_json(json_str: &str) -> Result<ImportedBundle, ConfigError> {
    let input: serde_json::Value = serde_json::from_str(json_str)?;

    // A root-level `settings` or `personalDates` key marks the input as a bundle,
    // so a malformed bundle is rejected instead of being read as legacy settings.
    if input.get("settings").is_some() || input.get("personalDates").is_some() {
        let mut bundle: ExportBundle = serde_json::from_value(input)?;
        bundle.settings.migrate();
        bundle.settings.validate()?;
        for date in &bundle.personal_dates {
            date.validate()?;
        }
        return Ok(ImportedBundle {
            settings: bundle.settings,
            personal_dates: Some(bundle.personal_dates),
        });
    }

    // Legacy Settings-only format.
    let mut settings: Settings = serde_json::from_value(input)?;
    settings.migrate();
    settings.validate()?;
    Ok(ImportedBundle {
        settings,
        personal_dates: None,
    })
}
