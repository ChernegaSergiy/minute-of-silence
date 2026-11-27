//! Hardware device models for local network discovery.

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Discovered standalone Minute of Silence hardware device on the local network.
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
}
