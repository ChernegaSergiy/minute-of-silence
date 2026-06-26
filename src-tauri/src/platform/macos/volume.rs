//! macOS volume control via native CoreAudio C API.

use crate::error::{AppError, Result};
use std::ffi::c_void;
use std::ptr;

type OSStatus = i32;
type AudioObjectID = u32;

const NO_ERR: OSStatus = 0;
const K_AUDIO_OBJECT_UNKNOWN: AudioObjectID = 0;
const K_AUDIO_OBJECT_SYSTEM_OBJECT: AudioObjectID = 1;

const fn fcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16 | (c as u32) << 8 | d as u32
}

const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE: u32 = fcc(b'd', b'f', b'l', b'o');
const K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR: u32 = fcc(b'v', b'o', b'l', b'm');
const K_AUDIO_DEVICE_PROPERTY_MUTE: u32 = fcc(b'm', b'u', b't', b'e');
const K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL: u32 = fcc(b'g', b'l', b'o', b'b');
const K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT: u32 = fcc(b'o', b'u', b't', b'p');

#[repr(C)]
struct AudioObjectPropertyAddress {
    m_selector: u32,
    m_scope: u32,
    m_element: u32,
}

unsafe extern "C" {
    fn AudioObjectGetPropertyData(
        object: AudioObjectID,
        address: *const AudioObjectPropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        data_size: *mut u32,
        data: *mut c_void,
    ) -> OSStatus;

    fn AudioObjectSetPropertyData(
        object: AudioObjectID,
        address: *const AudioObjectPropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        data_size: u32,
        data: *const c_void,
    ) -> OSStatus;
}

fn default_output_device() -> Option<AudioObjectID> {
    let mut device_id = K_AUDIO_OBJECT_UNKNOWN;
    let mut size = size_of::<AudioObjectID>() as u32;
    let address = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE,
        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        m_element: 0,
    };
    let status = unsafe {
        AudioObjectGetPropertyData(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            ptr::null(),
            &mut size,
            &mut device_id as *mut _ as *mut c_void,
        )
    };
    (status == NO_ERR && device_id != K_AUDIO_OBJECT_UNKNOWN).then_some(device_id)
}

pub fn get_volume() -> Result<u8> {
    let device_id = default_output_device()
        .ok_or_else(|| AppError::Platform("No default output device".into()))?;

    let mut volume: f32 = 0.0;
    let mut size = size_of::<f32>() as u32;
    let address = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR,
        m_scope: K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT,
        m_element: 0,
    };

    let mut status = unsafe {
        AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            ptr::null(),
            &mut size,
            &mut volume as *mut _ as *mut c_void,
        )
    };

    if status != NO_ERR {
        let fallback = AudioObjectPropertyAddress {
            m_selector: K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR,
            m_scope: K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT,
            m_element: 0,
        };
        status = unsafe {
            AudioObjectGetPropertyData(
                device_id,
                &fallback,
                0,
                ptr::null(),
                &mut size,
                &mut volume as *mut _ as *mut c_void,
            )
        };
    }

    if status == NO_ERR {
        Ok((volume * 100.0) as u8)
    } else {
        Err(AppError::Platform("Failed to get volume".into()))
    }
}

pub fn set_volume(level: u8) -> Result<()> {
    let device_id = default_output_device()
        .ok_or_else(|| AppError::Platform("No default output device".into()))?;

    let volume = (level.min(100) as f32) / 100.0;
    let size = size_of::<f32>() as u32;
    let address = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR,
        m_scope: K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT,
        m_element: 0,
    };

    let mut status = unsafe {
        AudioObjectSetPropertyData(
            device_id,
            &address,
            0,
            ptr::null(),
            size,
            &volume as *const _ as *const c_void,
        )
    };

    if status != NO_ERR {
        let fallback = AudioObjectPropertyAddress {
            m_selector: K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR,
            m_scope: K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT,
            m_element: 0,
        };
        status = unsafe {
            AudioObjectSetPropertyData(
                device_id,
                &fallback,
                0,
                ptr::null(),
                size,
                &volume as *const _ as *const c_void,
            )
        };
    }

    if status == NO_ERR {
        Ok(())
    } else {
        Err(AppError::Platform("Failed to set volume".into()))
    }
}

pub fn is_muted() -> Result<bool> {
    let device_id = default_output_device()
        .ok_or_else(|| AppError::Platform("No default output device".into()))?;

    let mut mute: u32 = 0;
    let mut size = size_of::<u32>() as u32;
    let address = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
        m_scope: K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT,
        m_element: 0,
    };

    let mut status = unsafe {
        AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            ptr::null(),
            &mut size,
            &mut mute as *mut _ as *mut c_void,
        )
    };

    if status != NO_ERR {
        let fallback = AudioObjectPropertyAddress {
            m_selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
            m_scope: K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT,
            m_element: 0,
        };
        status = unsafe {
            AudioObjectGetPropertyData(
                device_id,
                &fallback,
                0,
                ptr::null(),
                &mut size,
                &mut mute as *mut _ as *mut c_void,
            )
        };
    }

    if status == NO_ERR {
        Ok(mute != 0)
    } else {
        Err(AppError::Platform("Failed to get mute state".into()))
    }
}

pub fn set_mute(mute: bool) -> Result<()> {
    let device_id = default_output_device()
        .ok_or_else(|| AppError::Platform("No default output device".into()))?;

    let mute_val: u32 = if mute { 1 } else { 0 };
    let size = size_of::<u32>() as u32;
    let address = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
        m_scope: K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT,
        m_element: 0,
    };

    let mut status = unsafe {
        AudioObjectSetPropertyData(
            device_id,
            &address,
            0,
            ptr::null(),
            size,
            &mute_val as *const _ as *const c_void,
        )
    };

    if status != NO_ERR {
        let fallback = AudioObjectPropertyAddress {
            m_selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
            m_scope: K_AUDIO_DEVICE_PROPERTY_SCOPE_OUTPUT,
            m_element: 0,
        };
        status = unsafe {
            AudioObjectSetPropertyData(
                device_id,
                &fallback,
                0,
                ptr::null(),
                size,
                &mute_val as *const _ as *const c_void,
            )
        };
    }

    if status == NO_ERR {
        Ok(())
    } else {
        Err(AppError::Platform("Failed to set mute".into()))
    }
}
