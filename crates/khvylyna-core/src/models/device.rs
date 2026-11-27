//! Hardware device models for local network discovery.

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Category of a discovered Minute of Silence device on the local network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    /// Standalone ESP32 hardware device (`minute-of-silence-device`).
    Hardware,
    /// Desktop application (Windows, Linux, macOS).
    Desktop,
    /// Android mobile application.
    Android,
    /// Unclassified or legacy device without a `kind` TXT record.
    #[default]
    Unknown,
}

/// Discovered Minute of Silence device on the local network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDevice {
    /// Advertised hostname (e.g. "khvylyna.local").
    pub hostname: String,
    /// Local IPv4 address of the device (e.g. "192.168.1.105").
    pub ip: String,
    /// Port for device communication (e.g. 80 for HTTP).
    pub port: u16,
    /// Category of the discovered device (hardware ESP32, desktop, Android, etc.).
    #[serde(default)]
    pub kind: DeviceKind,
    /// Optional human-readable device name advertised via DNS-SD TXT record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
