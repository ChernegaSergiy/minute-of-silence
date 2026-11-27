//! Core domain models and declarations.

pub mod dates;
pub mod device;
pub mod presets;
pub mod settings;
pub mod status;

pub use dates::PersonalDate;
pub use device::DiscoveredDevice;
pub use presets::{AnnouncementVoice, AnthemVoice, AudioPreset, PresetBlock, VoiceCapabilities};
pub use settings::{CONFIG_SCHEMA_VERSION, Settings, UiTheme, validate_blocks};
pub use status::{AppInitialState, StatusSnapshot};
