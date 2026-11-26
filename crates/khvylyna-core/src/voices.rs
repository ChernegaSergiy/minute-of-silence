//! Audio asset manifest and block resolution for announcement voices.

use alloc::vec::Vec;
use core::time::Duration;

use crate::{AnnouncementVoice, PresetBlock, VoiceCapabilities};

/// Standard duration of audio assets for an announcement voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceDurations {
    /// Duration of the opening announcement recording.
    pub announcement: Duration,
    /// Duration of the bell recording.
    pub bell: Duration,
    /// Duration of the ending recording, if the voice has one.
    pub ending: Option<Duration>,
}

/// Audio files and properties that belong to a single announcement voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceAssets {
    /// Spoken announcement that opens the ceremony.
    pub announcement: &'static str,
    /// Bell used by the bell presets.
    pub bell: &'static str,
    /// Metronome played during the minute of silence.
    pub metronome: &'static str,
    /// Recording that closes the ceremony, if the voice has one.
    pub ending: Option<&'static str>,
    /// Whether the announcement already starts with the bell.
    pub announcement_includes_bell: bool,
    /// Default durations measured from the official bundled recordings.
    pub durations: VoiceDurations,
}

impl VoiceAssets {
    /// Returns the audio assets and metadata for the given voice.
    pub const fn for_voice(voice: AnnouncementVoice) -> Self {
        match voice {
            AnnouncementVoice::BohdanHdal => Self {
                announcement: "announcement.ogg",
                bell: "bell.ogg",
                metronome: "metronome.ogg",
                ending: Some("ending.ogg"),
                announcement_includes_bell: false,
                durations: VoiceDurations {
                    announcement: Duration::from_millis(2950),
                    bell: Duration::from_millis(3650),
                    ending: Some(Duration::from_millis(1420)),
                },
            },
            AnnouncementVoice::SoniaSotnyk => Self {
                announcement: "announcement_sotnyk.ogg",
                bell: "bell.ogg",
                metronome: "metronome.ogg",
                ending: Some("ending_sotnyk.ogg"),
                announcement_includes_bell: false,
                durations: VoiceDurations {
                    announcement: Duration::from_millis(36980),
                    bell: Duration::from_millis(3650),
                    ending: Some(Duration::from_millis(2990)),
                },
            },
            AnnouncementVoice::DaniaKhomutovskyi => Self {
                announcement: "announcement_khomutovskyi.ogg",
                bell: "bell.ogg",
                metronome: "metronome.ogg",
                ending: Some("ending_khomutovskyi.ogg"),
                announcement_includes_bell: false,
                durations: VoiceDurations {
                    announcement: Duration::from_millis(30860),
                    bell: Duration::from_millis(3650),
                    ending: Some(Duration::from_millis(1710)),
                },
            },
            AnnouncementVoice::RadioBg => Self {
                announcement: "announcement_radio_bg.ogg",
                bell: "bell.ogg",
                metronome: "metronome.ogg",
                ending: Some("ending_radio_bg.ogg"),
                announcement_includes_bell: false,
                durations: VoiceDurations {
                    announcement: Duration::from_millis(37200),
                    bell: Duration::from_millis(3650),
                    ending: Some(Duration::from_millis(15880)),
                },
            },
            AnnouncementVoice::VinnytsiaCity => Self {
                announcement: "announcement_vinnytsia.ogg",
                bell: "bell_vinnytsia.ogg",
                metronome: "metronome_vinnytsia.ogg",
                ending: None,
                announcement_includes_bell: true,
                durations: VoiceDurations {
                    announcement: Duration::from_millis(37480),
                    bell: Duration::from_millis(9080),
                    ending: None,
                },
            },
        }
    }
}

/// Returns what every announcement voice can play, for user interface and capability checks.
pub fn voice_capabilities() -> Vec<VoiceCapabilities> {
    AnnouncementVoice::ALL
        .into_iter()
        .map(|voice| {
            let assets = VoiceAssets::for_voice(voice);
            VoiceCapabilities {
                voice,
                has_ending: assets.ending.is_some(),
                announcement_includes_bell: assets.announcement_includes_bell,
            }
        })
        .collect()
}

/// Returns the blocks the selected voice can actually play.
///
/// A block the voice does not have (an ending, for example) is dropped, and so
/// is a bell right before an announcement that already starts with one.
pub fn effective_blocks(blocks: &[PresetBlock], voice: AnnouncementVoice) -> Vec<PresetBlock> {
    let assets = VoiceAssets::for_voice(voice);

    blocks
        .iter()
        .enumerate()
        .filter(|(index, block)| match block {
            PresetBlock::Ending => assets.ending.is_some(),
            PresetBlock::Bell => {
                !(assets.announcement_includes_bell
                    && blocks.get(index + 1) == Some(&PresetBlock::Announcement))
            }
            _ => true,
        })
        .map(|(_, block)| *block)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VOICES: [AnnouncementVoice; 5] = [
        AnnouncementVoice::BohdanHdal,
        AnnouncementVoice::SoniaSotnyk,
        AnnouncementVoice::DaniaKhomutovskyi,
        AnnouncementVoice::RadioBg,
        AnnouncementVoice::VinnytsiaCity,
    ];

    #[test]
    fn vinnytsia_has_no_ending() {
        assert_eq!(
            VoiceAssets::for_voice(AnnouncementVoice::VinnytsiaCity).ending,
            None
        );
    }

    #[test]
    fn default_voice_files_match_the_original_names() {
        let assets = VoiceAssets::for_voice(AnnouncementVoice::BohdanHdal);
        assert_eq!(assets.announcement, "announcement.ogg");
        assert_eq!(assets.bell, "bell.ogg");
        assert_eq!(assets.metronome, "metronome.ogg");
        assert_eq!(assets.ending, Some("ending.ogg"));
    }

    #[test]
    fn only_vinnytsia_announcement_includes_the_bell() {
        for voice in VOICES {
            assert_eq!(
                VoiceAssets::for_voice(voice).announcement_includes_bell,
                voice == AnnouncementVoice::VinnytsiaCity,
                "{voice:?}"
            );
        }
    }

    #[test]
    fn effective_blocks_drop_a_missing_ending() {
        use PresetBlock::*;

        let blocks = [Announcement, Metronome, Ending];
        assert_eq!(
            effective_blocks(&blocks, AnnouncementVoice::VinnytsiaCity),
            [Announcement, Metronome]
        );
        assert_eq!(
            effective_blocks(&blocks, AnnouncementVoice::BohdanHdal),
            blocks
        );
    }

    #[test]
    fn effective_blocks_drop_a_bell_before_an_announcement_with_a_bell() {
        use PresetBlock::*;

        let blocks = [Bell, Announcement, Metronome];
        assert_eq!(
            effective_blocks(&blocks, AnnouncementVoice::VinnytsiaCity),
            [Announcement, Metronome]
        );
        assert_eq!(
            effective_blocks(&blocks, AnnouncementVoice::BohdanHdal),
            blocks
        );
    }

    #[test]
    fn effective_blocks_keep_the_other_bells_of_vinnytsia() {
        use PresetBlock::*;

        for blocks in [
            &[Bell, Silence, Bell][..],
            &[Announcement, Silence, Bell][..],
            &[Bell, Metronome, Bell][..],
        ] {
            assert_eq!(
                effective_blocks(blocks, AnnouncementVoice::VinnytsiaCity),
                blocks
            );
        }
    }

    #[test]
    fn capabilities_cover_every_voice_and_match_the_manifest() {
        let capabilities = voice_capabilities();
        assert_eq!(capabilities.len(), VOICES.len());

        for voice in VOICES {
            let entry = capabilities
                .iter()
                .find(|entry| entry.voice == voice)
                .unwrap_or_else(|| panic!("no capabilities for {voice:?}"));
            let assets = VoiceAssets::for_voice(voice);
            assert_eq!(entry.has_ending, assets.ending.is_some(), "{voice:?}");
            assert_eq!(
                entry.announcement_includes_bell, assets.announcement_includes_bell,
                "{voice:?}"
            );
        }
    }
}
