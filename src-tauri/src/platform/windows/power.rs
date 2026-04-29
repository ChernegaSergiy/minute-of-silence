//! Listen for `WM_POWERBROADCAST` events so the scheduler can detect
//! whether the PC woke from sleep after 09:00.

use windows::Win32::UI::WindowsAndMessaging::{PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND};

#[allow(dead_code)]
pub fn is_resume_event(wparam: usize) -> bool {
    wparam == PBT_APMRESUMESUSPEND as usize || wparam == PBT_APMRESUMEAUTOMATIC as usize
}
