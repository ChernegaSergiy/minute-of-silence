//! Audio presets, blocks, voices, and capabilities.

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Selected announcement voice/version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementVoice {
    BohdanHdal,
    SoniaSotnyk,
    DaniaKhomutovskyi,
    RadioBg,
    VinnytsiaCity,
}

/// The name of a voice that was removed from the app reads as the default voice,
/// so the rest of an old settings file stays readable. Any other unknown name is
/// an error, so a misspelled voice in an imported file is not silently replaced.
impl<'de> Deserialize<'de> for AnnouncementVoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Ok(match name.as_str() {
            "bohdan_hdal" => Self::BohdanHdal,
            "sonia_sotnyk" => Self::SoniaSotnyk,
            "dania_khomutovskyi" => Self::DaniaKhomutovskyi,
            "radio_bg" => Self::RadioBg,
            "vinnytsia_city" => Self::VinnytsiaCity,
            // Removed from the app.
            "air_alert" => Self::BohdanHdal,
            other => {
                return Err(serde::de::Error::unknown_variant(
                    other,
                    &[
                        "bohdan_hdal",
                        "sonia_sotnyk",
                        "dania_khomutovskyi",
                        "radio_bg",
                        "vinnytsia_city",
                    ],
                ));
            }
        })
    }
}

impl AnnouncementVoice {
    /// Every announcement voice, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::BohdanHdal,
        Self::SoniaSotnyk,
        Self::DaniaKhomutovskyi,
        Self::RadioBg,
        Self::VinnytsiaCity,
    ];
}

/// What an announcement voice can play.
///
/// Lets the interface disable the blocks a voice does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "camelCase")]
pub struct VoiceCapabilities {
    /// The voice these capabilities belong to.
    pub voice: AnnouncementVoice,
    /// Whether the voice has a closing recording.
    pub has_ending: bool,
    /// Whether the announcement already starts with the bell.
    pub announcement_includes_bell: bool,
}

/// Selected anthem performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "snake_case")]
pub enum AnthemVoice {
    Default,
    MykhailoKhoma,
    OleksandrPonomarov,
}

/// Pre-configured audio presets for the ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "snake_case")]
pub enum AudioPreset {
    /// Voice announcement + metronome
    VoiceMetronome,
    /// Metronome only (no voice)
    MetronomeOnly,
    /// Voice announcement + 60 s silence + bell
    VoiceSilenceBell,
    /// Voice announcement + 60 s silence
    VoiceSilence,
    /// Voice announcement + metronome + national anthem
    VoiceMetronomeAnthem,
    /// Voice announcement + metronome + ending
    VoiceMetronomeEnding,
    /// Metronome + national anthem
    MetronomeAnthem,
    /// Bell + 60 s silence + bell
    BellSilenceBell,
    /// Bell + metronome + bell
    BellMetronomeBell,
    /// Silence only (no audio, just visual overlay)
    Silence,
}

impl AudioPreset {
    pub fn has_anthem(self) -> bool {
        matches!(self, Self::VoiceMetronomeAnthem | Self::MetronomeAnthem)
    }

    /// Returns the blocks this built-in preset is made of, in playback order.
    pub fn blocks(self) -> &'static [PresetBlock] {
        use PresetBlock::*;

        match self {
            Self::VoiceMetronome => &[Announcement, Metronome],
            Self::MetronomeOnly => &[Metronome],
            Self::VoiceSilenceBell => &[Announcement, Silence, Bell],
            Self::VoiceSilence => &[Announcement, Silence],
            Self::VoiceMetronomeAnthem => &[Announcement, Metronome, Anthem],
            Self::VoiceMetronomeEnding => &[Announcement, Metronome, Ending],
            Self::MetronomeAnthem => &[Metronome, Anthem],
            Self::BellSilenceBell => &[Bell, Silence, Bell],
            Self::BellMetronomeBell => &[Bell, Metronome, Bell],
            Self::Silence => &[Silence],
        }
    }
}

/// A building block of an audio preset.
///
/// Every file a block plays comes from the single selected announcement voice,
/// so a preset never mixes recordings of different voices. The anthem is the
/// exception: it has its own voice setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export_to = "types.gen.ts"))]
#[serde(rename_all = "snake_case")]
pub enum PresetBlock {
    /// Spoken announcement of the selected voice.
    Announcement,
    /// Bell of the selected voice.
    Bell,
    /// Metronome of the selected voice.
    Metronome,
    /// One minute of silence.
    Silence,
    /// Closing recording of the selected voice. Skipped when the voice has none.
    Ending,
    /// National anthem in the selected anthem voice.
    Anthem,
}

impl PresetBlock {
    /// Whether the block plays a recording. Only silence does not.
    pub fn is_audible(self) -> bool {
        self != Self::Silence
    }
}
