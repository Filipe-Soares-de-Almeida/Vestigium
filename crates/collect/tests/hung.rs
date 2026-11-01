use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::hung_pids;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG,
    PM_REMOVE, PeekMessageW, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    WS_VISIBLE,
};
use windows::core::w;

#[test]
fn hung_pids_returns_on_any_desktop() {
    let first = hung_pids(std::process::id());
    let second = hung_pids(std::process::id());

    for pids in [&first, &second] {
        assert!(pids.windows(2).all(|w| w[0] < w[1]), "sorted and unique");
        assert!(!pids.contains(&std::process::id()));
        assert!(!pids.contains(&0));
    }
}

/// Opens a visible window on a new thread, answers for 1 s, then ignores messages for `hang`.
fn hang_a_window(hang: Duration) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // SAFETY: a plain window of a class registered here, owned and destroyed by this thread.
        unsafe {
            let instance = GetModuleHandleW(None).unwrap();
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                lpszClassName: w!("VestigiumTestHang"),
                ..Default::default()
            };
            RegisterClassW(&class);
            let window = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("VestigiumTestHang"),
                w!("hung window test"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                200,
                100,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .unwrap();
            let answer_until = Instant::now() + Duration::from_secs(1);
            let mut msg = MSG::default();
            while Instant::now() < answer_until {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    DispatchMessageW(&msg);
                }
                thread::sleep(Duration::from_millis(15));
            }
            thread::sleep(hang);
            let _ = DestroyWindow(window);
        }
    })
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: forwards every message unchanged.
    unsafe { DefWindowProcW(hwnd, msg, w, l) }
}

#[test]
#[ignore = "needs an interactive desktop and takes 9 s"]
fn own_hung_window_is_found_and_excluded() {
    let me = std::process::id();
    let hanging = hang_a_window(Duration::from_secs(8));
    thread::sleep(Duration::from_secs(7));

    let anyone = hung_pids(u32::MAX);
    let others = hung_pids(me);
    hanging.join().unwrap();

    assert!(anyone.contains(&me), "the hung test window was not found");
    assert!(
        !others.contains(&me),
        "the caller's own process is excluded"
    );
}
