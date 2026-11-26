use khvylyna_core::AnnouncementVoice;
use khvylyna_core::voices::VoiceAssets;
use std::path::Path;

#[test]
fn every_listed_file_exists() {
    let audio_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("audio");
    for voice in AnnouncementVoice::ALL {
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
