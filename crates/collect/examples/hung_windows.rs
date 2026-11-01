//! Every 2 s, lists each top-level window Windows reports as not responding (class, owner,
//! visibility, the window behind a ghost) next to what `hung_pids` keeps: `hung_windows --seconds <s>`.

use std::thread;
use std::time::Duration;

use vestigium_collect::hung_pids;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId, IsHungAppWindow,
    IsWindowVisible,
};
use windows::core::{BOOL, s, w};

type HungFromGhost = unsafe extern "system" fn(HWND) -> HWND;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds: u64 = std::env::args()
        .skip_while(|a| a != "--seconds")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(40);
    // SAFETY: user32 is loaded in every GUI-capable process; the export is looked up by name.
    let hung_from_ghost: Option<HungFromGhost> = unsafe {
        let user32 = GetModuleHandleW(w!("user32.dll"))?;
        GetProcAddress(user32, s!("HungWindowFromGhostWindow"))
            .map(|f| std::mem::transmute::<_, HungFromGhost>(f))
    };
    println!(
        "HungWindowFromGhostWindow: {}",
        if hung_from_ghost.is_some() {
            "exported"
        } else {
            "missing"
        }
    );

    for tick in 0..seconds / 2 {
        thread::sleep(Duration::from_secs(2));
        let hung: Vec<HWND> = top_level()
            .into_iter()
            // SAFETY: a plain query on a window handle from EnumWindows.
            .filter(|&hwnd| unsafe { IsHungAppWindow(hwnd) }.as_bool())
            .collect();
        println!(
            "{:>3} s  {} hung, hung_pids {:?}",
            (tick + 1) * 2,
            hung.len(),
            hung_pids(std::process::id())
        );
        for hwnd in hung {
            println!("        {}", describe(hwnd));
            if class(hwnd) == "Ghost"
                && let Some(resolve) = hung_from_ghost
            {
                // SAFETY: the export takes a window handle and returns a handle or null.
                let original = unsafe { resolve(hwnd) };
                if original.is_invalid() {
                    println!("          ghost of: (none)");
                } else {
                    println!("          ghost of: {}", describe(original));
                }
            }
        }
    }
    Ok(())
}

fn top_level() -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::new();
    // SAFETY: the callback only pushes into the Vec passed through `lparam`, alive for the call.
    let _ = unsafe { EnumWindows(Some(collect), LPARAM((&raw mut windows) as isize)) };
    windows
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the Vec handed to EnumWindows above.
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    windows.push(hwnd);
    true.into()
}

fn class(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    // SAFETY: writes at most the buffer's length.
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

fn describe(hwnd: HWND) -> String {
    let mut pid = 0u32;
    let mut title = [0u16; 256];
    // SAFETY: plain queries on a window handle; each writes inside its own buffer.
    let (visible, len) = unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        (
            IsWindowVisible(hwnd).as_bool(),
            GetWindowTextW(hwnd, &mut title),
        )
    };
    format!(
        "class={} pid={pid} visible={visible} title={:?}",
        class(hwnd),
        String::from_utf16_lossy(&title[..len.max(0) as usize])
    )
}
