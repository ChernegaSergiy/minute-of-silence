//! Manage snap autostart by writing/removing a .desktop file in
//! $SNAP_USER_DATA/.config/autostart — the official snapd mechanism.
//!
//! snapd's `snap userd --autostart` scans that directory at session
//! start and launches apps whose desktop-file name matches the
//! `autostart:` field declared in snapcraft.yaml.

pub fn manage(enable: bool) {
    let snap_user_data = match std::env::var("SNAP_USER_DATA") {
        Ok(v) => v,
        Err(_) => return, // not running as a snap
    };

    let autostart_dir = std::path::PathBuf::from(&snap_user_data).join(".config/autostart");
    // Must match `autostart: minute-of-silence.desktop` in snapcraft.yaml
    let desktop_path = autostart_dir.join("minute-of-silence.desktop");

    if enable {
        if let Err(e) = std::fs::create_dir_all(&autostart_dir) {
            log::warn!(
                "snap autostart: cannot create dir {:?}: {}",
                autostart_dir,
                e
            );
            return;
        }
        // Exec= must use the snap command name (no absolute path).
        // snapd rewrites it through the confinement wrapper automatically.
        let content = "[Desktop Entry]\n\
            Type=Application\n\
            Name=Хвилина мовчання\n\
            Exec=minute-of-silence --hidden\n\
            Hidden=false\n\
            NoDisplay=false\n\
            X-GNOME-Autostart-enabled=true\n";
        match std::fs::write(&desktop_path, content) {
            Ok(_) => log::info!("snap autostart: enabled ({:?})", desktop_path),
            Err(e) => log::warn!("snap autostart: write failed: {}", e),
        }
    } else {
        match std::fs::remove_file(&desktop_path) {
            Ok(_) => log::info!("snap autostart: disabled ({:?})", desktop_path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {} // already absent
            Err(e) => log::warn!("snap autostart: remove failed: {}", e),
        }
    }
}
