//! Backend audio playback engine.

use rodio::{Decoder, DeviceSinkBuilder, Player};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::core::settings::{AnnouncementVoice, AnthemVoice, AudioPreset};
use crate::core::voices::VoiceAssets;
use crate::error::{AppError, Result};

/// Audio engine for ceremony playback.
/// This acts as a class responsible for all audio operations.
#[derive(Debug)]
pub struct AudioEngine {
    app_handle: AppHandle,
    stop_counter: AtomicU64,
}

/// A single step in a preset playback sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    File(String),
    Pause(Duration),
    Wait,           // wait for the current player to drain
    Anthem(String), // like File but emits anthem-start/anthem-end events
}

impl AudioEngine {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            stop_counter: AtomicU64::new(0),
        }
    }

    /// Stop any current playback by incrementing the counter.
    pub fn stop(&self) {
        self.stop_counter.fetch_add(1, Ordering::SeqCst);
    }

    fn is_stopped(&self, start_counter: u64) -> bool {
        self.stop_counter.load(Ordering::SeqCst) > start_counter
    }

    fn sleep_interruptible(&self, duration: Duration, start_counter: u64) -> bool {
        let start = Instant::now();
        while start.elapsed() < duration {
            if self.is_stopped(start_counter) {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn wait_player_interruptible(&self, player: &Player, start_counter: u64) -> bool {
        while !player.empty() {
            if self.is_stopped(start_counter) {
                player.stop();
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }

    pub fn get_duration(&self, filename: &str) -> Result<Duration> {
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::formats::TrackType;
        use symphonia::core::formats::probe::Hint;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;

        let path = self.get_path(filename)?;
        let mss = MediaSourceStream::new(Box::new(File::open(&path)?), Default::default());

        let mut hint = Hint::new();
        hint.with_extension("ogg");

        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| AppError::Audio(format!("Failed to probe audio file: {e}")))?;

        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| AppError::Audio(format!("No audio track in {filename}")))?;

        let (duration, time_base) = track
            .duration
            .zip(track.time_base)
            .ok_or_else(|| AppError::Audio(format!("No duration metadata in {filename}")))?;

        let secs =
            duration.get() as f64 * time_base.numer.get() as f64 / time_base.denom.get() as f64;
        Ok(Duration::from_secs_f64(secs))
    }

    pub fn play_preset(
        &self,
        preset: AudioPreset,
        volume: u8,
        voice: AnnouncementVoice,
        anthem_voice: AnthemVoice,
    ) -> Result<()> {
        let start_counter = self.stop_counter.load(Ordering::SeqCst);

        let sink = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| AppError::Audio(format!("Failed to open audio stream: {e}")))?;

        let mixer = sink.mixer();
        let player = Player::connect_new(mixer);

        let volume_float = volume as f32 / 100.0;
        player.set_volume(volume_float);

        // Build the playback step sequence once and execute it.
        let steps = preset_steps(preset, voice, anthem_voice);

        for step in steps {
            if self.is_stopped(start_counter) {
                return Ok(());
            }

            match step {
                Step::File(fname) => {
                    let path = self.get_path(&fname)?;
                    if let Ok(source) = Decoder::new(BufReader::new(File::open(&path)?)) {
                        player.append(source);
                    }
                }
                Step::Pause(dur) => {
                    if self.sleep_interruptible(dur, start_counter) {
                        return Ok(());
                    }
                }
                Step::Wait => {
                    if self.wait_player_interruptible(&player, start_counter) {
                        return Ok(());
                    }
                }
                Step::Anthem(fname) => {
                    let anthem_path = self.get_path(&fname)?;
                    let _ = self.app_handle.emit("anthem-start", ());
                    if let Ok(source) = Decoder::new(BufReader::new(File::open(&anthem_path)?)) {
                        player.append(source);
                    }
                    if self.wait_player_interruptible(&player, start_counter) {
                        let _ = self.app_handle.emit("anthem-end", ());
                        return Ok(());
                    }
                    let _ = self.app_handle.emit("anthem-end", ());
                }
            }
        }

        self.wait_player_interruptible(&player, start_counter);
        Ok(())
    }

    /// Estimate total duration of a preset by summing file durations and explicit pauses.
    pub fn estimate_preset_duration(
        &self,
        preset: AudioPreset,
        voice: AnnouncementVoice,
        anthem_voice: AnthemVoice,
    ) -> Result<Duration> {
        let mut total = Duration::from_secs(0);
        let steps = preset_steps(preset, voice, anthem_voice);
        for step in &steps {
            match step {
                Step::File(f) | Step::Anthem(f) => {
                    let dur = self.get_duration(f)?;
                    total = total.checked_add(dur).unwrap_or(total);
                }
                Step::Pause(d) => {
                    total = total.checked_add(*d).unwrap_or(total);
                }
                Step::Wait => {
                    // no-op: waits are covered by file durations
                }
            }
        }
        Ok(total)
    }

    pub fn get_announcement_filename(&self, voice: AnnouncementVoice) -> String {
        VoiceAssets::for_voice(voice).announcement.to_string()
    }

    /// Returns the bell file used by the bell presets for the selected voice.
    pub fn get_bell_filename(&self, voice: AnnouncementVoice) -> String {
        VoiceAssets::for_voice(voice).bell.to_string()
    }

    /// Resolves the absolute path to an audio resource using Tauri's path resolver.
    /// Works on all platforms: EXE, MSI, MSIX, AppImage, Snap.
    fn get_path(&self, filename: &str) -> Result<PathBuf> {
        let resource_path = format!("audio/{}", filename);

        // 1. Try Tauri's standard resource resolver
        let tauri_path = self
            .app_handle
            .path()
            .resolve(&resource_path, tauri::path::BaseDirectory::Resource)
            .ok();

        // 2. Try relative to the executable (common in Snap bin/)
        let exe_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("audio").join(filename)));

        // 3. Try hardcoded Snap layout path (as a last resort)
        let layout_path = Some(PathBuf::from(format!(
            "/usr/lib/minute-of-silence/audio/{}",
            filename
        )));

        let candidates = vec![tauri_path, exe_path, layout_path];

        for candidate in candidates.into_iter().flatten() {
            if candidate.exists() {
                log::info!("Found audio at: {:?}", candidate);
                return Ok(candidate);
            }
            log::debug!("Audio not found at: {:?}", candidate);
        }

        log::error!(
            "Audio resource {} not found in any candidate path",
            filename
        );
        Err(AppError::Audio(format!(
            "Audio file not found: {}",
            filename
        )))
    }
}

/// Return a list of ordered steps for the given preset.
fn preset_steps(
    preset: AudioPreset,
    voice: AnnouncementVoice,
    anthem_voice: AnthemVoice,
) -> Vec<Step> {
    use AudioPreset::*;

    let assets = VoiceAssets::for_voice(voice);
    let mut out = Vec::new();

    match preset {
        VoiceMetronome => {
            out.push(Step::File(assets.announcement.to_string()));
            out.push(Step::Wait);
            out.push(Step::Pause(Duration::from_secs(1)));
            out.push(Step::File(assets.metronome.to_string()));
        }
        MetronomeOnly => {
            out.push(Step::File(assets.metronome.to_string()));
        }
        VoiceSilenceBell => {
            out.push(Step::File(assets.announcement.to_string()));
            out.push(Step::Wait);
            out.push(Step::Pause(Duration::from_secs(60)));
            out.push(Step::File(assets.bell.to_string()));
        }
        VoiceSilence => {
            out.push(Step::File(assets.announcement.to_string()));
            out.push(Step::Wait);
            out.push(Step::Pause(Duration::from_secs(60)));
        }
        VoiceMetronomeAnthem => {
            out.push(Step::File(assets.announcement.to_string()));
            out.push(Step::Wait);
            out.push(Step::Pause(Duration::from_secs(1)));
            out.push(Step::File(assets.metronome.to_string()));
            out.push(Step::Wait);
            out.push(Step::Anthem(anthem_filename(anthem_voice)));
        }
        VoiceMetronomeEnding => {
            out.push(Step::File(assets.announcement.to_string()));
            out.push(Step::Wait);
            out.push(Step::Pause(Duration::from_secs(1)));
            out.push(Step::File(assets.metronome.to_string()));

            // Append the ending file only if one exists for the selected voice
            if let Some(ending_file) = assets.ending {
                out.push(Step::File(ending_file.to_string()));
            }
        }
        MetronomeAnthem => {
            out.push(Step::File(assets.metronome.to_string()));
            out.push(Step::Wait);
            out.push(Step::Anthem(anthem_filename(anthem_voice)));
        }
        BellSilenceBell => {
            out.push(Step::File(assets.bell.to_string()));
            out.push(Step::Wait);
            out.push(Step::Pause(Duration::from_secs(60)));
            out.push(Step::File(assets.bell.to_string()));
        }
        BellMetronomeBell => {
            out.push(Step::File(assets.bell.to_string()));
            out.push(Step::Wait);
            out.push(Step::Pause(Duration::from_secs(1)));
            out.push(Step::File(assets.metronome.to_string()));
            out.push(Step::File(assets.bell.to_string()));
        }
        Silence => {
            out.push(Step::Pause(Duration::from_secs(60)));
        }
    }

    out
}

/// Returns the national anthem recording for the selected anthem voice.
fn anthem_filename(voice: AnthemVoice) -> String {
    match voice {
        AnthemVoice::Default => "anthem.ogg".to_string(),
        AnthemVoice::MykhailoKhoma => "anthem_khoma.ogg".to_string(),
        AnthemVoice::OleksandrPonomarov => "anthem_ponomarov.ogg".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AudioPreset::*;

    const PRESETS: [AudioPreset; 10] = [
        VoiceMetronome,
        MetronomeOnly,
        VoiceSilenceBell,
        VoiceSilence,
        VoiceMetronomeAnthem,
        VoiceMetronomeEnding,
        MetronomeAnthem,
        BellSilenceBell,
        BellMetronomeBell,
        Silence,
    ];

    const VOICES: [AnnouncementVoice; 6] = [
        AnnouncementVoice::BohdanHdal,
        AnnouncementVoice::SoniaSotnyk,
        AnnouncementVoice::DaniaKhomutovskyi,
        AnnouncementVoice::RadioBg,
        AnnouncementVoice::AirAlert,
        AnnouncementVoice::VinnytsiaCity,
    ];

    fn file(name: &str) -> Step {
        Step::File(name.to_string())
    }

    fn pause(secs: u64) -> Step {
        Step::Pause(Duration::from_secs(secs))
    }

    fn anthem(name: &str) -> Step {
        Step::Anthem(name.to_string())
    }

    fn steps(preset: AudioPreset, voice: AnnouncementVoice) -> Vec<Step> {
        preset_steps(preset, voice, AnthemVoice::Default)
    }

    #[test]
    fn default_voice_preset_sequences() {
        let voice = AnnouncementVoice::BohdanHdal;
        let expected = [
            (
                VoiceMetronome,
                vec![
                    file("announcement.ogg"),
                    Step::Wait,
                    pause(1),
                    file("metronome.ogg"),
                ],
            ),
            (MetronomeOnly, vec![file("metronome.ogg")]),
            (
                VoiceSilenceBell,
                vec![
                    file("announcement.ogg"),
                    Step::Wait,
                    pause(60),
                    file("bell.ogg"),
                ],
            ),
            (
                VoiceSilence,
                vec![file("announcement.ogg"), Step::Wait, pause(60)],
            ),
            (
                VoiceMetronomeAnthem,
                vec![
                    file("announcement.ogg"),
                    Step::Wait,
                    pause(1),
                    file("metronome.ogg"),
                    Step::Wait,
                    anthem("anthem.ogg"),
                ],
            ),
            (
                VoiceMetronomeEnding,
                vec![
                    file("announcement.ogg"),
                    Step::Wait,
                    pause(1),
                    file("metronome.ogg"),
                    file("ending.ogg"),
                ],
            ),
            (
                MetronomeAnthem,
                vec![file("metronome.ogg"), Step::Wait, anthem("anthem.ogg")],
            ),
            (
                BellSilenceBell,
                vec![file("bell.ogg"), Step::Wait, pause(60), file("bell.ogg")],
            ),
            (
                BellMetronomeBell,
                vec![
                    file("bell.ogg"),
                    Step::Wait,
                    pause(1),
                    file("metronome.ogg"),
                    file("bell.ogg"),
                ],
            ),
            (Silence, vec![pause(60)]),
        ];

        assert_eq!(expected.len(), PRESETS.len());
        for (preset, sequence) in expected {
            assert_eq!(steps(preset, voice), sequence, "{preset:?}");
        }
    }

    #[test]
    fn vinnytsia_uses_its_own_bell_and_metronome() {
        let voice = AnnouncementVoice::VinnytsiaCity;

        assert_eq!(
            steps(BellSilenceBell, voice),
            vec![
                file("bell_vinnytsia.ogg"),
                Step::Wait,
                pause(60),
                file("bell_vinnytsia.ogg"),
            ]
        );
        assert_eq!(
            steps(BellMetronomeBell, voice),
            vec![
                file("bell_vinnytsia.ogg"),
                Step::Wait,
                pause(1),
                file("metronome_vinnytsia.ogg"),
                file("bell_vinnytsia.ogg"),
            ]
        );
        assert_eq!(
            steps(VoiceMetronomeEnding, voice),
            vec![
                file("announcement_vinnytsia.ogg"),
                Step::Wait,
                pause(1),
                file("metronome_vinnytsia.ogg"),
                file("bell_vinnytsia.ogg"),
            ]
        );
    }

    #[test]
    fn voice_without_an_ending_skips_it() {
        assert_eq!(
            steps(VoiceMetronomeEnding, AnnouncementVoice::AirAlert),
            vec![
                file("announcement_air_alert.ogg"),
                Step::Wait,
                pause(1),
                file("metronome.ogg"),
            ]
        );
    }

    #[test]
    fn anthem_voice_selects_the_anthem_file() {
        let voice = AnnouncementVoice::BohdanHdal;
        for (anthem_voice, name) in [
            (AnthemVoice::Default, "anthem.ogg"),
            (AnthemVoice::MykhailoKhoma, "anthem_khoma.ogg"),
            (AnthemVoice::OleksandrPonomarov, "anthem_ponomarov.ogg"),
        ] {
            let sequence = preset_steps(MetronomeAnthem, voice, anthem_voice);
            assert_eq!(sequence.last(), Some(&anthem(name)));
        }
    }

    #[test]
    fn presets_never_mix_files_of_different_voices() {
        for voice in VOICES {
            let assets = VoiceAssets::for_voice(voice);
            let allowed = [
                Some(assets.announcement),
                Some(assets.bell),
                Some(assets.metronome),
                assets.ending,
            ];
            for preset in PRESETS {
                for step in steps(preset, voice) {
                    if let Step::File(name) = step {
                        assert!(
                            allowed.contains(&Some(name.as_str())),
                            "{preset:?} with {voice:?} plays {name}, which is not in the voice's manifest"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_preset_has_steps() {
        for voice in VOICES {
            for preset in PRESETS {
                assert!(!steps(preset, voice).is_empty(), "{preset:?}/{voice:?}");
            }
        }
    }
}
