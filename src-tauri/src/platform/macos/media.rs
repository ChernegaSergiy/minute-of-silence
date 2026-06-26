//! Pause / resume media on macOS via MediaRemote private framework.
//! Uses the same system API that media keys (F7/F8/F9) use.

use crate::error::Result;
use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::ptr;
use std::sync::OnceLock;

const MR_COMMAND_PAUSE: i32 = 1;
const MR_COMMAND_PLAY: i32 = 0;

static MEDIA_REMOTE: OnceLock<Option<Library>> = OnceLock::new();

fn send_command(command: i32) {
    let lib = MEDIA_REMOTE.get_or_init(|| unsafe {
        Library::new("/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote").ok()
    });

    if let Some(lib) = lib {
        unsafe {
            let func: Symbol<unsafe extern "C" fn(i32, *const c_void)> =
                match lib.get(b"MRMediaRemoteSendCommand") {
                    Ok(f) => f,
                    Err(_) => return,
                };
            func(command, ptr::null());
        }
    }
}

pub async fn pause_all() -> Result<Vec<String>> {
    send_command(MR_COMMAND_PAUSE);
    Ok(vec!["__mr__".to_string()])
}

pub async fn resume_specific(players: Vec<String>) -> Result<()> {
    if players.is_empty() {
        return Ok(());
    }
    // Don't send kMRPlay if there's no active Now Playing session.
    // Otherwise, kMRPlay with nil originator falls through to the
    // default media app (Apple Music) and opens its window.
    if !super::now_playing::has_now_playing_session() {
        return Ok(());
    }
    send_command(MR_COMMAND_PLAY);
    Ok(())
}
