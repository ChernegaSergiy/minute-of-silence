unsafe extern "C" {
    fn macos_has_now_playing_session() -> bool;
}

pub fn has_now_playing_session() -> bool {
    unsafe { macos_has_now_playing_session() }
}
