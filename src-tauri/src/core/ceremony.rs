#[cfg(desktop)]
use crate::core::settings::PresetBlock;
use crate::platform::Platform;
use crate::state::{ActiveCeremony, AppState, CeremonyKind, CeremonyPhase, CeremonyRuntime};
#[cfg(desktop)]
use rust_i18n::t;
use serde_json::json;
use std::sync::{Arc, Mutex};
#[cfg(desktop)]
use tauri::WebviewWindowBuilder;
#[cfg(desktop)]
use tauri::{AppHandle, EventId, Listener};
use tauri::{Emitter, Manager};

static PREVIOUS_VOLUME: Mutex<Option<u8>> = Mutex::new(None);
static WAS_MUTED: Mutex<Option<bool>> = Mutex::new(None);
#[cfg(desktop)]
static PAUSED_PLAYERS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// One-shot anthem listeners of the running ceremony, removed when it finishes.
#[cfg(desktop)]
static ANTHEM_LISTENERS: Mutex<Vec<EventId>> = Mutex::new(Vec::new());

/// Orchestrator for the ceremony.
/// This class manages the sequence of events during the ceremony.
pub struct CeremonyManager {
    runtime: Arc<CeremonyRuntime>,
    platform: Box<dyn Platform>,
}

impl CeremonyManager {
    pub(crate) fn new(runtime: Arc<CeremonyRuntime>, platform: Box<dyn Platform>) -> Self {
        Self { runtime, platform }
    }

    pub async fn run_ceremony(&self, is_test: bool) {
        #[cfg(desktop)]
        let should_resume_players;
        let (
            should_pause_players,
            volume_priority,
            auto_unmute,
            target_volume,
            blocks,
            announcement_voice,
            anthem_voice,
        ) = {
            let inner = self.runtime.lock();
            #[cfg(desktop)]
            {
                should_resume_players = inner.settings.resume_after_ceremony;
            }
            (
                inner.settings.pause_other_players,
                inner.settings.volume_priority,
                inner.settings.auto_unmute,
                inner.settings.volume,
                inner.settings.blocks.clone(),
                inner.settings.announcement_voice,
                inner.settings.anthem_voice,
            )
        };

        // 1. Mark active
        let started = (!is_test).then(chrono::Local::now);
        let ceremony_id = {
            let mut inner = self.runtime.lock();
            if inner.active_ceremony.is_some() {
                log::info!("Ceremony trigger ignored: a ceremony is already running");
                return;
            }
            inner.ceremony_id_counter += 1;
            let ceremony_id = inner.ceremony_id_counter;
            inner.active_ceremony = Some(ActiveCeremony::new(
                ceremony_id,
                if is_test {
                    CeremonyKind::Test
                } else {
                    CeremonyKind::Scheduled
                },
            ));
            if started.is_some() {
                inner.last_activation = started;
            }
            ceremony_id
        };
        #[cfg(target_os = "android")]
        if let Some(started) = started {
            crate::platform::android::headless::remember_start(started);
        }
        // 2. Setup flag animation listeners if conditions are met
        #[cfg(desktop)]
        let should_show_flag = {
            let inner = self.runtime.lock();
            inner.settings.show_flag_animation
                && inner.settings.show_visual_overlay
                && blocks.contains(&PresetBlock::Anthem)
        };
        #[cfg(desktop)]
        if should_show_flag && let Some(app) = self.runtime.app() {
            let app_clone = app.clone();
            let start_listener = app.once("anthem-start", move |_| {
                let app = app_clone.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = WebviewWindowBuilder::new(
                        &app,
                        "flag-animation",
                        tauri::WebviewUrl::App(std::path::PathBuf::from("flag-animation.html")),
                    )
                    .title(t!("tray_tooltip").as_ref())
                    .fullscreen(true)
                    .decorations(false)
                    .always_on_top(false)
                    .skip_taskbar(false)
                    .build()
                    {
                        log::warn!("Failed to create flag animation window: {}", e);
                    }
                });
            });

            let app_clone2 = app.clone();
            let end_listener = app.once("anthem-end", move |_| {
                let app = app_clone2.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(window) = app.get_webview_window("flag-animation") {
                        let _ = window.close();
                    }
                });
            });

            if let Ok(mut listeners) = ANTHEM_LISTENERS.lock() {
                listeners.extend([start_listener, end_listener]);
            }
        }

        // 3. Notify UI with estimated duration (ms)
        let duration_ms: u64 =
            match self
                .runtime
                .audio
                .estimate_duration(&blocks, announcement_voice, anthem_voice)
            {
                Ok(d) => d.as_millis() as u64,
                Err(e) => {
                    log::warn!("Failed to estimate ceremony duration: {}", e);
                    60_000u64
                }
            };

        let payload = json!({
            "duration_ms": duration_ms,
            "is_test": is_test,
        });
        if let Some(app) = self.runtime.app() {
            let _ = app.emit("ceremony-start", payload);
        }

        // 3. Pause players
        #[cfg(desktop)]
        if should_pause_players {
            if let Ok(players) = self.platform.pause_media().await {
                if should_resume_players {
                    let mut lock = PAUSED_PLAYERS.lock().unwrap();
                    *lock = players;
                }
            }
        }

        // 4. Handle Volume and Mute (skip when nothing is played)
        let plays_audio = crate::core::voices::effective_blocks(&blocks, announcement_voice)
            .iter()
            .any(|block| block.is_audible());
        if auto_unmute && plays_audio {
            // Save mute state and unmute if necessary
            if let Ok(muted) = self.platform.is_muted() {
                if muted {
                    *WAS_MUTED.lock().unwrap() = Some(true);
                    let _ = self.platform.set_mute(false);
                }
            }
        }

        if volume_priority && plays_audio {
            // Save and set volume
            if let Ok(vol) = self.platform.get_volume() {
                *PREVIOUS_VOLUME.lock().unwrap() = Some(vol);
                let _ = self.platform.set_volume(target_volume);
            }
        }

        // 6. Play Audio (Stop previous first)
        self.runtime.audio.stop();
        let stop_baseline = self.runtime.audio.stop_baseline();
        // A stop pressed while the ceremony was still being set up is replayed here.
        // The baseline is taken first, so a stop that arrives after this check
        // still ends the playback through the counter.
        let stop_requested = {
            let mut inner = self.runtime.lock();
            let Some(active) = inner.active_ceremony.as_mut() else {
                return;
            };
            if active.id != ceremony_id {
                return;
            }
            let requested = active.stop_requested;
            active.stop_requested = false;
            active.phase = CeremonyPhase::Playing;
            requested
        };

        #[cfg(target_os = "android")]
        if should_pause_players && !stop_requested {
            crate::platform::android::media::request_focus(ceremony_id);
            let still_playing = self.runtime.lock().active_ceremony.is_some_and(|active| {
                active.id == ceremony_id && active.phase == CeremonyPhase::Playing
            });
            if !still_playing {
                crate::platform::android::media::release_focus(ceremony_id);
                return;
            }
        }

        let runtime = Arc::clone(&self.runtime);
        let platform_handle = crate::platform::get_platform();

        std::thread::spawn(move || {
            if stop_requested {
                log::info!("Ceremony stopped before playback started");
            } else if let Err(e) = runtime.audio.play_blocks(
                &blocks,
                target_volume,
                announcement_voice,
                anthem_voice,
                stop_baseline,
            ) {
                log::error!("Ceremony audio error: {}", e);
            }

            // 7. Finish
            tauri::async_runtime::spawn(async move {
                CeremonyManager::finish_ceremony(runtime, platform_handle, ceremony_id).await;
            });
        });
    }

    /// Drop the one-shot anthem listeners that did not fire, for example when
    /// the ceremony was stopped before the anthem. Removing a listener that
    /// already fired does nothing.
    #[cfg(desktop)]
    fn remove_anthem_listeners(app: &AppHandle) {
        let listeners = match ANTHEM_LISTENERS.lock() {
            Ok(mut lock) => std::mem::take(&mut *lock),
            Err(_) => return,
        };
        for id in listeners {
            app.unlisten(id);
        }
    }

    pub(crate) async fn finish_ceremony(
        runtime: Arc<CeremonyRuntime>,
        platform: Box<dyn Platform>,
        ceremony_id: u64,
    ) {
        #[cfg(desktop)]
        let should_resume;
        let (volume_priority, auto_unmute, pending_restart) = {
            let mut inner = runtime.lock();
            let Some(active) = inner.active_ceremony.as_mut() else {
                return;
            };
            if active.id != ceremony_id || active.phase == CeremonyPhase::Finishing {
                return;
            }
            active.phase = CeremonyPhase::Finishing;
            #[cfg(desktop)]
            {
                should_resume = inner.settings.resume_after_ceremony;
            }
            (
                inner.settings.volume_priority,
                inner.settings.auto_unmute,
                inner.pending_restart,
            )
        };
        let app = runtime.app();
        #[cfg(desktop)]
        if let Some(app) = &app {
            Self::remove_anthem_listeners(app);
        }

        // Restore volume and mute
        if volume_priority {
            let prev_vol = *PREVIOUS_VOLUME.lock().unwrap();
            if let Some(vol) = prev_vol {
                let _ = platform.set_volume(vol);
                *PREVIOUS_VOLUME.lock().unwrap() = None;
            }
        }

        if auto_unmute {
            let was_muted = *WAS_MUTED.lock().unwrap();
            if let Some(true) = was_muted {
                let _ = platform.set_mute(true);
                *WAS_MUTED.lock().unwrap() = None;
            }
        }

        // Restore media playback
        #[cfg(desktop)]
        if should_resume {
            let players = {
                let mut lock = PAUSED_PLAYERS.lock().unwrap();
                std::mem::take(&mut *lock)
            };
            if !players.is_empty() {
                let _ = platform.resume_media(players).await;
            }
        }
        #[cfg(target_os = "android")]
        crate::platform::android::media::release_focus(ceremony_id);

        // Clear skip_date after successful ceremony
        if let Some(app) = &app {
            let state = app.state::<AppState>();
            if state.lock().settings.skip_date.is_some() {
                if let Err(e) = state.update_settings(|s| {
                    s.skip_date = None;
                }) {
                    log::warn!("Failed to clear skip_date: {e}");
                }
            }
        }

        {
            let mut inner = runtime.lock();
            if inner
                .active_ceremony
                .is_some_and(|ceremony| ceremony.id == ceremony_id)
            {
                inner.active_ceremony = None;
            }
        }

        if let Some(app) = app {
            let _ = app.emit("ceremony-end", ());
            if pending_restart {
                log::info!("Executing deferred app restart after ceremony.");
                app.restart();
            }
        }
    }
}
