//! Application settings definition, defaults, migrations, and validation.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use crate::models::presets::{AnnouncementVoice, AnthemVoice, AudioPreset, PresetBlock};

/// Current configuration schema version.
pub const CONFIG_SCHEMA_VERSION: u32 = 2;

fn default_schema_version() -> u32 {
    CONFIG_SCHEMA_VERSION
}

fn default_ntp_server() -> String {
    "pool.ntp.org".to_string()
}

/// User-configurable application settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Schema version for configuration export and migrations.
    #[serde(default = "default_schema_version")]
    pub version: u32,

    /// Enable daily activation at 09:00.
    pub ceremony_enabled: bool,

    /// Enable app autostart when the system boots.
    pub autostart_enabled: bool,

    /// Run ceremony only on weekdays (Mon-Fri).
    pub weekdays_only: bool,

    /// Selected audio preset (1–10).
    ///
    /// Superseded by [`Settings::blocks`]. It is only read to migrate settings
    /// saved before schema version 2.
    pub preset: AudioPreset,

    /// Blocks the ceremony audio is made of, in playback order.
    ///
    /// Every file comes from the single selected `announcement_voice`. Settings
    /// saved before schema version 2 have no blocks; [`Settings::migrate`] fills
    /// them in from `preset`.
    #[serde(default = "alloc::vec::Vec::new")]
    pub blocks: Vec<PresetBlock>,

    /// Master volume for the ceremony audio (0–100).
    pub volume: u8,

    /// Pause other media players before the ceremony.
    pub pause_other_players: bool,

    /// Automatically resume paused players after the ceremony.
    pub resume_after_ceremony: bool,

    /// Show a visual overlay window when the ceremony starts.
    pub show_visual_overlay: bool,

    /// Show the flag animation window when the ceremony starts.
    pub show_flag_animation: bool,

    /// Show personal dates on nearby days with weighted proximity.
    pub show_nearby_personal_dates: bool,

    /// Use system time instead of NTP.
    pub system_time_only: bool,

    /// Prioritize app volume over system controls.
    pub volume_priority: bool,

    /// Automatically unmute system if muted during ceremony.
    pub auto_unmute: bool,

    /// NTP server hostname (used when system_time_only is false).
    pub ntp_server: String,

    /// If the system wakes from sleep after 09:00, activate if within this
    /// many minutes of the scheduled time (0 = never activate late).
    pub late_start_grace_minutes: u8,

    /// Enable reminder notifications.
    pub reminder_enabled: bool,

    /// Show a system notification N minutes before the ceremony (0 = immediately).
    /// Valid range: 0–10.
    pub reminder_minutes_before: u8,

    /// Selected announcement voice/version.
    pub announcement_voice: AnnouncementVoice,

    /// Selected anthem voice/performance.
    pub anthem_voice: AnthemVoice,

    /// Follow the OS theme when true; otherwise use `ui_theme`.
    pub use_system_theme: bool,

    /// Manual UI theme when `use_system_theme` is false.
    pub ui_theme: UiTheme,

    /// ID of the last personal date shown via nearby-days algorithm.
    #[serde(default)]
    pub last_shown_nearby_date_id: Option<String>,

    /// Date to skip the next ceremony (one-time skip). Persisted to disk.
    #[serde(default)]
    pub skip_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "lowercase")]
pub enum UiTheme {
    Light,
    Dark,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: CONFIG_SCHEMA_VERSION,
            ceremony_enabled: true,
            autostart_enabled: true,
            weekdays_only: false,
            preset: AudioPreset::VoiceMetronome,
            blocks: AudioPreset::VoiceMetronome.blocks().to_vec(),
            volume: 80,
            pause_other_players: true,
            resume_after_ceremony: false,
            show_visual_overlay: true,
            show_flag_animation: false,
            show_nearby_personal_dates: false,
            system_time_only: false,
            volume_priority: false,
            auto_unmute: false,
            ntp_server: default_ntp_server(),
            late_start_grace_minutes: 1,
            reminder_enabled: false,
            reminder_minutes_before: 5,
            announcement_voice: AnnouncementVoice::BohdanHdal,
            anthem_voice: AnthemVoice::Default,
            use_system_theme: true,
            ui_theme: UiTheme::Light,
            last_shown_nearby_date_id: None,
            skip_date: None,
        }
    }
}

impl Settings {
    /// Validates that the settings schema version is supported.
    pub fn validate_version(&self) -> Result<(), ConfigError> {
        if self.version == 0 {
            return Err(ConfigError::InvalidVersion(0));
        }
        if self.version > CONFIG_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                found: self.version,
                max_supported: CONFIG_SCHEMA_VERSION,
            });
        }
        Ok(())
    }

    /// Brings settings saved with an older schema up to date.
    ///
    /// Settings written before schema version 2 only know the selected `preset`,
    /// so their blocks are taken from it. Settings that already have blocks keep
    /// them. Invalid versions are left as they are so validation still rejects them.
    ///
    /// The Vinnytsia voice has no spoken ending. What it used to play as one was
    /// its closing bell, so an ending of that voice becomes a bell.
    pub fn migrate(&mut self) {
        if self.blocks.is_empty() {
            self.blocks = self.preset.blocks().to_vec();
        }
        if (1..CONFIG_SCHEMA_VERSION).contains(&self.version)
            && self.announcement_voice == AnnouncementVoice::VinnytsiaCity
        {
            for block in &mut self.blocks {
                if *block == PresetBlock::Ending {
                    *block = PresetBlock::Bell;
                }
            }
        }
        if (1..CONFIG_SCHEMA_VERSION).contains(&self.version) {
            self.version = CONFIG_SCHEMA_VERSION;
        }
    }

    /// Validates that settings conform to the schema version and field bounds.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_version()?;

        if let Err(message) = validate_blocks(&self.blocks) {
            return Err(ConfigError::InvalidField {
                field: "blocks",
                message,
            });
        }

        if self.volume > 100 {
            return Err(ConfigError::InvalidField {
                field: "volume",
                message: "Volume must be between 0 and 100",
            });
        }

        if self.late_start_grace_minutes > 5 {
            return Err(ConfigError::InvalidField {
                field: "lateStartGraceMinutes",
                message: "Late start grace minutes must be between 0 and 5",
            });
        }

        if self.reminder_minutes_before > 10 {
            return Err(ConfigError::InvalidField {
                field: "reminderMinutesBefore",
                message: "Reminder minutes before must be between 0 and 10",
            });
        }

        if self.ntp_server.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: "ntpServer",
                message: "NTP server hostname cannot be empty",
            });
        }

        Ok(())
    }
}

/// Checks that a list of blocks has the layout the constructor can build: an
/// optional opening bell, an optional announcement, exactly one main part (a
/// metronome or silence) and at most one closing block (ending, bell or anthem),
/// in this order.
pub fn validate_blocks(blocks: &[PresetBlock]) -> Result<(), &'static str> {
    use PresetBlock::*;

    if blocks.is_empty() {
        return Err("At least one block is required");
    }

    let mut rest = blocks;
    for optional in [Bell, Announcement] {
        if rest.first() == Some(&optional) {
            rest = &rest[1..];
        }
    }

    match rest.split_first() {
        Some((Metronome | Silence, closing)) => match closing {
            [] | [Ending | Bell | Anthem] => Ok(()),
            _ => Err("Only one ending, bell or anthem may close a preset"),
        },
        _ => Err(
            "Blocks must be an optional bell, an optional announcement, \
             a metronome or silence, and an optional closing block, in this order",
        ),
    }
}
