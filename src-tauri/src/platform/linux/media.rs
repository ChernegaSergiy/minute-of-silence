//! Pause media players on Linux using D-Bus (MPRIS).

use crate::error::{AppError, Result};
use zbus::proxy;

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_service = "org.mpris.MediaPlayer2.spotify",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MediaPlayer2Player {
    fn pause(&self) -> zbus::Result<()>;
    fn play(&self) -> zbus::Result<()>;
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;
}

pub async fn pause_all() -> Result<()> {
    let result = tokio::task::spawn_blocking(|| {
        let conn =
            zbus::blocking::Connection::session().map_err(|e| AppError::Platform(e.to_string()))?;
        let dbus = zbus::blocking::fdo::DBusProxy::new(&conn)
            .map_err(|e| AppError::Platform(e.to_string()))?;
        let names = dbus
            .list_names()
            .map_err(|e| AppError::Platform(e.to_string()))?;

        for name in names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                let player = MediaPlayer2PlayerProxyBlocking::builder(&conn)
                    .destination(name.as_str())
                    .map_err(|e| AppError::Platform(e.to_string()))?
                    .build()
                    .map_err(|e| AppError::Platform(e.to_string()))?;

                if let Ok(status) = player.playback_status() {
                    if status == "Playing" {
                        let _ = player.pause();
                    }
                }
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| AppError::Platform(e.to_string()))?;

    result
}
