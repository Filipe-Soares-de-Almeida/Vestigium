use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PostMessageW,
    PostQuitMessage, RegisterClassW, SetTimer, TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_CLOSE,
    WM_DISPLAYCHANGE, WM_ENDSESSION, WM_POWERBROADCAST, WM_QUERYENDSESSION, WM_SETTINGCHANGE,
    WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::w;

use crate::power;

type Handler = Box<dyn Fn() + Send + Sync>;

static ON_END_SESSION: OnceLock<Handler> = OnceLock::new();
/// The hidden window, as an address so any thread can post to it.
static SESSION_WINDOW: OnceLock<usize> = OnceLock::new();
/// Posted to make the message loop run once, so the tray shows a change made elsewhere.
const WM_APP_WAKE: u32 = WM_APP + 1;
/// How often the message loop runs by itself, so the tray shows a sampler that stopped writing.
#[cfg(not(test))]
const TRAY_REFRESH_MS: u32 = 15_000;
#[cfg(test)]
const TRAY_REFRESH_MS: u32 = 50;
const TRAY_TIMER_ID: usize = 1;
/// Set when Windows reports a changed setting or display: theme, scale or monitors.
static DISPLAY_CHANGED: AtomicBool = AtomicBool::new(false);

/// Creates the hidden top-level window that hears logoff, shutdown, suspend and resume; `on_end` runs before the session ends.
pub fn watch_session_end(on_end: impl Fn() + Send + Sync + 'static) -> windows::core::Result<HWND> {
    let _ = ON_END_SESSION.set(Box::new(on_end));
    // SAFETY: the module handle of this executable.
    let instance = unsafe { GetModuleHandleW(None)? };
    let class = WNDCLASSW {
        lpfnWndProc: Some(session_proc),
        hInstance: instance.into(),
        lpszClassName: w!("VestigiumAgentSession"),
        ..Default::default()
    };
    // SAFETY: registers a class whose strings are static.
    unsafe { RegisterClassW(&class) };
    // SAFETY: a never-shown top-level window; message-only windows miss WM_QUERYENDSESSION.
    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("VestigiumAgentSession"),
            w!("Vestigium agent"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )?
    };
    let _ = SESSION_WINDOW.set(window.0 as usize);
    // SAFETY: a timer of the window just made; its WM_TIMER needs no handler, only the loop's turn.
    unsafe { SetTimer(Some(window), TRAY_TIMER_ID, TRAY_REFRESH_MS, None) };
    Ok(window)
}

/// Makes the message loop run once, from any thread; does nothing before the window exists.
pub fn wake() {
    if let Some(&window) = SESSION_WINDOW.get() {
        // SAFETY: posts a message carrying no pointers to this process's own window.
        let _ = unsafe {
            PostMessageW(
                Some(HWND(window as *mut c_void)),
                WM_APP_WAKE,
                WPARAM(0),
                LPARAM(0),
            )
        };
    }
}

/// Runs the message loop of this thread until `quit`, calling `after_each` after every message.
pub fn pump(mut after_each: impl FnMut()) {
    let mut msg = MSG::default();
    // SAFETY: the standard loop over this thread's queue; GetMessageW returns 0 on WM_QUIT, -1 on error.
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
        // SAFETY: `msg` was just filled by GetMessageW.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        after_each();
    }
}

/// True once after each setting or display change Windows reported.
pub fn display_changed() -> bool {
    DISPLAY_CHANGED.swap(false, Ordering::Relaxed)
}

/// Ends `pump` on this thread.
pub fn quit() {
    // SAFETY: posts WM_QUIT to the calling thread's queue.
    unsafe { PostQuitMessage(0) };
}

unsafe extern "system" fn session_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_QUERYENDSESSION => LRESULT(1),
        // A close request ends the loop as the tray's Exit does; the window lives until the process ends.
        WM_CLOSE => {
            quit();
            LRESULT(0)
        }
        WM_ENDSESSION => {
            if wparam.0 != 0
                && let Some(on_end) = ON_END_SESSION.get()
            {
                on_end();
                // The sampler is stopped; the process ends too, as a restart request asks.
                quit();
            }
            LRESULT(0)
        }
        WM_POWERBROADCAST => {
            power::on_broadcast(wparam.0);
            LRESULT(1)
        }
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE => {
            DISPLAY_CHANGED.store(true, Ordering::Relaxed);
            wake();
            // SAFETY: the default handling still runs, unchanged.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        // SAFETY: forwards everything else unchanged.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_close_request_ends_the_message_loop() {
        let (done, finished) = mpsc::channel();
        std::thread::spawn(move || {
            let window = watch_session_end(|| {}).unwrap();
            // SAFETY: posts a message carrying no pointers to the window this thread just made.
            unsafe { PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0)) }.unwrap();
            pump(|| {});
            let _ = done.send(());
        });
        assert!(finished.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    #[test]
    fn the_message_loop_runs_by_itself_on_a_timer() {
        let (done, finished) = mpsc::channel();
        std::thread::spawn(move || {
            watch_session_end(|| {}).unwrap();
            // Nothing is posted: only the window's timer can give the loop a turn.
            pump(quit);
            let _ = done.send(());
        });
        assert!(finished.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    #[test]
    fn a_session_end_ends_the_message_loop_after_the_handler() {
        let (done, finished) = mpsc::channel();
        std::thread::spawn(move || {
            let window = watch_session_end(|| {}).unwrap();
            // SAFETY: posts a message carrying no pointers to the window this thread just made.
            unsafe { PostMessageW(Some(window), WM_ENDSESSION, WPARAM(1), LPARAM(1)) }.unwrap();
            pump(|| {});
            let _ = done.send(());
        });
        assert!(finished.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    #[test]
    fn the_installer_closes_the_window_this_module_makes() {
        let hooks = include_str!("../../ui/src-tauri/windows/hooks.nsh");
        assert!(hooks.contains("FindWindow $0 \"VestigiumAgentSession\""));
    }
}
