use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, GetSysColorBrush};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG,
    PM_REMOVE, PeekMessageW, RegisterClassW, TranslateMessage, WINDOW_EX_STYLE, WNDCLASSW,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};
use windows::core::w;

use crate::sleep_until_stopped;

/// How long the window answers before it stops and after it recovers.
const RESPONSIVE: Duration = Duration::from_secs(2);

/// Opens a window, answers for 2 s, ignores every message for `seconds`, then recovers and closes.
pub fn hang(seconds: u64) -> Result<(), String> {
    let window = create()?;
    println!(
        "Window open; it stops responding in {} s. Click it to see Windows mark it Not Responding.",
        RESPONSIVE.as_secs()
    );
    pump_for(RESPONSIVE);
    println!("Ignoring messages for {seconds} s.");
    sleep_until_stopped(Duration::from_secs(seconds));
    println!("Responding again.");
    pump_for(RESPONSIVE);
    // SAFETY: destroys the window this thread created; it may already be gone if the user closed it.
    let _ = unsafe { DestroyWindow(window) };
    Ok(())
}

fn create() -> Result<HWND, String> {
    // SAFETY: the module handle of this executable.
    let instance = unsafe { GetModuleHandleW(None) }.map_err(|e| e.to_string())?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: w!("VestigiumScenarioHang"),
        // SAFETY: a system brush, which needs no release.
        hbrBackground: unsafe { GetSysColorBrush(COLOR_WINDOW) },
        ..Default::default()
    };
    // SAFETY: registers a class whose strings are static.
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(windows::core::Error::from_thread().to_string());
    }
    // SAFETY: a plain top-level window of the class registered above, owned by this thread.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("VestigiumScenarioHang"),
            w!("Vestigium scenario: hang-window"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            480,
            200,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .map_err(|e| e.to_string())
}

/// Handles this thread's messages for `span` without blocking on an empty queue.
fn pump_for(span: Duration) {
    let deadline = Instant::now() + span;
    let mut msg = MSG::default();
    while Instant::now() < deadline {
        // SAFETY: reads this thread's queue into `msg` without waiting.
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            // SAFETY: `msg` was just filled by PeekMessageW.
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        thread::sleep(Duration::from_millis(15));
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: forwards every message unchanged.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
