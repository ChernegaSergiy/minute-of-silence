//! Audio asset manifest and block filtering for announcement voices.
//!
//! Re-exports domain logic from `khvylyna_core::voices`.

pub use khvylyna_core::voices::{VoiceAssets, effective_blocks, voice_capabilities};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::AnnouncementVoice;
    use std::path::Path;

    const VOICES: [AnnouncementVoice; 5] = [
        AnnouncementVoice::BohdanHdal,
        AnnouncementVoice::SoniaSotnyk,
        AnnouncementVoice::DaniaKhomutovskyi,
        AnnouncementVoice::RadioBg,
        AnnouncementVoice::VinnytsiaCity,
    ];

    #[test]
    fn every_listed_file_exists() {
        let audio_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("audio");
        for voice in VOICES {
            let assets = VoiceAssets::for_voice(voice);
            let files = [
                Some(assets.announcement),
                Some(assets.bell),
                Some(assets.metronome),
                assets.ending,
            ];
            for file in files.into_iter().flatten() {
                assert!(
                    audio_dir.join(file).is_file(),
                    "{voice:?}: missing audio file {file}"
                );
            }
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
