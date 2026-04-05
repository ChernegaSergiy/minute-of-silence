//! Windows-specific platform integrations.
//!
//! Exposes two sub-modules:
//! * `media`  — pause / resume system-wide media playback.
//! * `volume` — control system volume.

pub mod volume {
    use crate::error::{AppError, Result};
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

    pub fn get_volume() -> Result<u8> {
        unsafe {
            let endpoint = get_endpoint()?;
            let volume = endpoint
                .GetMasterVolumeLevelScalar()
                .map_err(|e| AppError::Platform(e.to_string()))?;
            Ok((volume * 100.0) as u8)
        }
    }

    pub fn set_volume(level: u8) -> Result<()> {
        unsafe {
            let endpoint = get_endpoint()?;
            let clamped = (level as f32 / 100.0).clamp(0.0, 1.0);
            endpoint
                .SetMasterVolumeLevelScalar(clamped, std::ptr::null())
                .map_err(|e| AppError::Platform(e.to_string()))?;

            Ok(())
        }
    }

    pub fn is_muted() -> Result<bool> {
        unsafe {
            let endpoint = get_endpoint()?;
            let muted = endpoint
                .GetMute()
                .map_err(|e| AppError::Platform(e.to_string()))?;
            Ok(muted.as_bool())
        }
    }

    pub fn set_mute(mute: bool) -> Result<()> {
        unsafe {
            let endpoint = get_endpoint()?;
            endpoint
                .SetMute(mute, std::ptr::null())
                .map_err(|e| AppError::Platform(e.to_string()))?;
            Ok(())
        }
    }

    fn get_endpoint() -> Result<IAudioEndpointVolume> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| AppError::Platform(e.to_string()))?;

            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| AppError::Platform(e.to_string()))?;

            let endpoint: IAudioEndpointVolume = device
                .Activate(CLSCTX_INPROC_SERVER, None)
                .map_err(|e| AppError::Platform(e.to_string()))?;

            Ok(endpoint)
        }
    }
}

pub mod media {
    use windows::core::GUID;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, AudioSessionStateActive, IAudioClient, IAudioSessionControl,
        IAudioSessionManager2, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

    use crate::error::{AppError, Result};

    static IID_IAUDIOCLIENT: GUID = GUID::from_values(
        0x1cb9ad4c,
        0xdbfa,
        0x4c32,
        [0xb1, 0x78, 0xc2, 0xf5, 0x68, 0xa7, 0x03, 0xb2],
    );

    fn get_audio_client(session: &IAudioSessionControl) -> Result<IAudioClient> {
        unsafe {
            let raw = windows::core::Interface::as_raw(session);
            let vtable = (*(raw as *const *const _)) as *const *const std::ffi::c_void;
            let get_service: extern "system" fn(
                *const std::ffi::c_void,
                *const GUID,
                *mut *mut std::ffi::c_void,
            )
                -> windows::Win32::Foundation::WIN32_ERROR = std::mem::transmute(*vtable.offset(3));
            let mut audio_client_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let hr = get_service(raw, &IID_IAUDIOCLIENT, &mut audio_client_ptr);
            if hr != windows::Win32::Foundation::WIN32_ERROR(0) {
                return Err(AppError::Platform(format!("GetService failed: {:?}", hr)));
            }
            windows::core::Type::from_abi(audio_client_ptr)
                .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))
        }
    }

    pub fn pause_all() -> Result<()> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| AppError::Platform(e.to_string()))?;

            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| AppError::Platform(e.to_string()))?;

            let session_manager: IAudioSessionManager2 = device
                .Activate(CLSCTX_INPROC_SERVER, None)
                .map_err(|e| AppError::Platform(e.to_string()))?;

            let session_enumerator = session_manager
                .GetSessionEnumerator()
                .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?;

            let count = session_enumerator
                .GetCount()
                .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?;

            for i in 0..count {
                let session: IAudioSessionControl = session_enumerator
                    .GetSession(i)
                    .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?;

                let state = session
                    .GetState()
                    .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?;

                if state == AudioSessionStateActive {
                    let audio_client = get_audio_client(&session)?;
                    audio_client
                        .Stop()
                        .map_err(|e: windows::core::Error| AppError::Platform(e.to_string()))?;
                }
            }

            Ok(())
        }
    }
}

pub mod power {
    //! Listen for `WM_POWERBROADCAST` events so the scheduler can detect
    //! whether the PC woke from sleep after 09:00.

    use windows::Win32::UI::WindowsAndMessaging::{PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND};

    #[allow(dead_code)]
    pub fn is_resume_event(wparam: usize) -> bool {
        wparam == PBT_APMRESUMESUSPEND as usize || wparam == PBT_APMRESUMEAUTOMATIC as usize
    }
}
