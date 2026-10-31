use std::sync::OnceLock;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PostQuitMessage,
    RegisterClassW, TranslateMessage, WINDOW_EX_STYLE, WM_ENDSESSION, WM_QUERYENDSESSION,
    WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::w;

type Handler = Box<dyn Fn() + Send + Sync>;

static ON_END_SESSION: OnceLock<Handler> = OnceLock::new();

/// Creates the hidden top-level window that hears logoff and shutdown; `on_end` runs before the session ends.
pub fn watch_session_end(on_end: impl Fn() + Send + Sync + 'static) -> windows::core::Result<HWND> {
    let _ = ON_END_SESSION.set(Box::new(on_end));
    // SAFETY: the module handle of this executable.
    let instance = unsafe { GetModuleHandleW(None)? };
    let class = WNDCLASSW {
        lpfnWndProc: Some(session_proc),
        hInstance: instance.into(),
        lpszClassName: w!("WatchMyPCAgentSession"),
        ..Default::default()
    };
    // SAFETY: registers a class whose strings are static.
    unsafe { RegisterClassW(&class) };
    // SAFETY: a never-shown top-level window; message-only windows miss WM_QUERYENDSESSION.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("WatchMyPCAgentSession"),
            w!("WatchMyPC agent"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
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
        WM_ENDSESSION => {
            if wparam.0 != 0
                && let Some(on_end) = ON_END_SESSION.get()
            {
                on_end();
            }
            LRESULT(0)
        }
        // SAFETY: forwards everything else unchanged.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
