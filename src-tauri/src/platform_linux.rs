//! Linux platform integrations.
//!
//! On Linux we use the MPRIS D-Bus interface to pause/resume media players.

pub mod media {
    use crate::error::{AppError, Result};
    use std::process::Command;

    pub fn pause_all() -> Result<()> {
        pause_all_via_mpris().or_else(|_| pause_all_via_xdotool())
    }

    pub fn resume_all() -> Result<()> {
        resume_all_via_mpris().or_else(|_| resume_all_via_xdotool())
    }

    fn pause_all_via_mpris() -> Result<()> {
        let output = Command::new("dbus-send")
            .args([
                "--print-reply",
                "--dest=org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus.ListNames",
            ])
            .output()
            .map_err(|e| AppError::Io(e))?;

        if !output.status.success() {
            return Err(AppError::Platform("Failed to list D-Bus names".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let players: Vec<&str> = stdout
            .lines()
            .filter(|line| line.contains("org.mpris.MediaPlayer2"))
            .filter_map(|line| {
                line.split('"')
                    .nth(1)
                    .filter(|s| !s.contains("org.freedesktop.DBus"))
            })
            .collect();

        if players.is_empty() {
            log::info!("No MPRIS media players found");
            return Err(AppError::Platform("No MPRIS players found".into()));
        }

        for player in &players {
            log::info!("Pausing MPRIS player: {}", player);
            let _ = Command::new("dbus-send")
                .args([
                    "--print-reply",
                    &format!("--dest={}", player),
                    "/org/mpris/MediaPlayer2",
                    "org.mpris.MediaPlayer2.Player.Pause",
                ])
                .output();
        }

        Ok(())
    }

    fn resume_all_via_mpris() -> Result<()> {
        let output = Command::new("dbus-send")
            .args([
                "--print-reply",
                "--dest=org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus.ListNames",
            ])
            .output()
            .map_err(|e| AppError::Io(e))?;

        if !output.status.success() {
            return Err(AppError::Platform("Failed to list D-Bus names".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let players: Vec<&str> = stdout
            .lines()
            .filter(|line| line.contains("org.mpris.MediaPlayer2"))
            .filter_map(|line| {
                line.split('"')
                    .nth(1)
                    .filter(|s| !s.contains("org.freedesktop.DBus"))
            })
            .collect();

        for player in &players {
            log::info!("Resuming MPRIS player: {}", player);
            let _ = Command::new("dbus-send")
                .args([
                    "--print-reply",
                    &format!("--dest={}", player),
                    "/org/mpris/MediaPlayer2",
                    "org.mpris.MediaPlayer2.Player.Play",
                ])
                .output();
        }

        Ok(())
    }

    fn pause_all_via_xdotool() -> Result<()> {
        let status = Command::new("xdotool")
            .args(["key", "--window", "0", "XF86AudioPlay"])
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(AppError::Platform(format!(
                "xdotool exited with status {s}"
            ))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::warn!("xdotool not found; skipping media key");
                Ok(())
            }
            Err(e) => Err(AppError::Io(e)),
        }
    }

    fn resume_all_via_xdotool() -> Result<()> {
        pause_all_via_xdotool()
    }
}
