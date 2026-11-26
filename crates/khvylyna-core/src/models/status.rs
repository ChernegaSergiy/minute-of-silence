//! Runtime status snapshot models.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::models::dates::PersonalDate;
use crate::models::presets::VoiceCapabilities;
use crate::models::settings::Settings;

/// Serialisable runtime status snapshot for the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct StatusSnapshot {
    pub ceremony_active: bool,
    pub ceremony_is_test: bool,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub ceremony_id: Option<u64>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub ceremony_elapsed_ms: Option<u64>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub ceremony_duration_ms: Option<u64>,
    pub skip_tomorrow: bool,
    pub last_activation: Option<String>,
    pub last_ntp_sync: Option<String>,
}

/// Combined initial application state for atomic startup hydration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct AppInitialState {
    pub settings: Settings,
    pub personal_dates: Vec<PersonalDate>,
    pub status: StatusSnapshot,
    pub version: String,
    /// What each announcement voice can play.
    pub voices: Vec<VoiceCapabilities>,
    pub supports_visual_overlay: bool,
    pub supports_media_resumption: bool,
}
