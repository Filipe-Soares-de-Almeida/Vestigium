use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsHungAppWindow, IsWindowVisible,
};
use windows::core::BOOL;

/// Class of the copy Windows draws over a hung window; it belongs to dwm.exe, not to the app.
const GHOST_CLASS: &str = "Ghost";

/// Processes with a top-level window on screen that Windows reports as not responding, `own_pid` left out.
pub fn hung_pids(own_pid: u32) -> Vec<u32> {
    let mut pids: Vec<u32> = top_level_windows()
        .into_iter()
        // SAFETY: plain queries on handles EnumWindows just returned; a closed window only answers false.
        .filter(|&hwnd| unsafe {
            IsWindowVisible(hwnd).as_bool() && IsHungAppWindow(hwnd).as_bool()
        })
        .filter(|&hwnd| on_screen(false, is_cloaked(hwnd), &class_name(hwnd)))
        .filter_map(owner)
        .filter(|&pid| pid != 0 && pid != own_pid)
        .collect();
    pids.sort_unstable();
    pids.dedup();
    pids
}

/// Whether a visible window is one the user sees as the app itself: not cloaked, not a ghost.
fn on_screen(hidden: bool, cloaked: bool, class: &str) -> bool {
    !hidden && !cloaked && class != GHOST_CLASS
}

fn top_level_windows() -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::with_capacity(256);
    // SAFETY: the callback only pushes into the Vec passed through `lparam`, alive for the call.
    let _ = unsafe { EnumWindows(Some(push_window), LPARAM((&raw mut windows) as isize)) };
    windows
}

unsafe extern "system" fn push_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the Vec handed to EnumWindows in `top_level_windows`.
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    windows.push(hwnd);
    true.into()
}

/// Windows that exist but are not drawn, such as those of suspended Store apps.
fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    // SAFETY: DWMWA_CLOAKED writes one u32 into `cloaked`.
    let read = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&raw mut cloaked).cast::<c_void>(),
            size_of::<u32>() as u32,
        )
    };
    read.is_ok() && cloaked != 0
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    // SAFETY: writes at most the buffer's length.
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

fn owner(hwnd: HWND) -> Option<u32> {
    let mut pid = 0u32;
    // SAFETY: writes the owning process id into `pid`.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_cloaked_and_ghost_windows_are_skipped() {
        assert!(on_screen(false, false, "VestigiumScenarioHang"));
        assert!(!on_screen(true, false, "IME"));
        assert!(!on_screen(false, true, "Windows.UI.Core.CoreWindow"));
        assert!(!on_screen(false, false, "Ghost"));
    }
}
