#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
mod is_msix;

#[cfg(target_os = "windows")]
pub use is_msix::is_msix_package;

use crate::error::Result;

#[async_trait::async_trait]
pub trait Platform: Send + Sync {
    fn get_volume(&self) -> Result<u8>;
    fn set_volume(&self, level: u8) -> Result<()>;
    fn is_muted(&self) -> Result<bool>;
    fn set_mute(&self, mute: bool) -> Result<()>;
    async fn pause_media(&self) -> Result<()>;
}

#[cfg(target_os = "windows")]
pub struct WindowsPlatform;

#[cfg(target_os = "windows")]
#[async_trait::async_trait]
impl Platform for WindowsPlatform {
    fn get_volume(&self) -> Result<u8> {
        crate::platform::windows::volume::get_volume()
    }
    fn set_volume(&self, level: u8) -> Result<()> {
        crate::platform::windows::volume::set_volume(level)
    }
    fn is_muted(&self) -> Result<bool> {
        crate::platform::windows::volume::is_muted()
    }
    fn set_mute(&self, mute: bool) -> Result<()> {
        crate::platform::windows::volume::set_mute(mute)
    }
    async fn pause_media(&self) -> Result<()> {
        crate::platform::windows::media::pause_all().await
    }
}

#[cfg(target_os = "linux")]
pub struct LinuxPlatform;

#[cfg(target_os = "linux")]
#[async_trait::async_trait]
impl Platform for LinuxPlatform {
    fn get_volume(&self) -> Result<u8> {
        crate::platform::linux::volume::get_volume()
    }
    fn set_volume(&self, level: u8) -> Result<()> {
        crate::platform::linux::volume::set_volume(level)
    }
    fn is_muted(&self) -> Result<bool> {
        crate::platform::linux::volume::is_muted()
    }
    fn set_mute(&self, mute: bool) -> Result<()> {
        crate::platform::linux::volume::set_mute(mute)
    }
    async fn pause_media(&self) -> Result<()> {
        crate::platform::linux::media::pause_all().await
    }
}

pub fn get_platform() -> Box<dyn Platform> {
    #[cfg(target_os = "windows")]
    return Box::new(WindowsPlatform);
    #[cfg(target_os = "linux")]
    return Box::new(LinuxPlatform);
}

// is_resume_event is defined in platform/windows/power.rs
// Linux does not need it currently.
