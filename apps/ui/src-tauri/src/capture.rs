use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
use windows::Win32::UI::Shell::{
    SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{PCWSTR, w};

use crate::agent::UiError;

/// Capture lengths the window accepts, in minutes.
pub const CAPTURE_MINUTES: std::ops::RangeInclusive<u64> = 1..=60;

/// A deep capture under way: when it started and when it should end, Unix epoch milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CaptureRun {
    pub started_ms: i64,
    pub ends_ms: i64,
}

/// What the window hears when the helper exits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaptureFinished {
    /// The captures the watcher stored, newest last.
    pub ids: Vec<i64>,
    pub exit_code: Option<u32>,
}

/// The helper next to this window: the installed name, then the development build's.
pub fn sibling_helper() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    ["Vestigium.Helper.exe", "vestigium-helper.exe"]
        .iter()
        .map(|name| exe.with_file_name(name))
        .find(|path| path.is_file())
}

/// `<data>\captures\<start ms>.capture`, beside the watcher's database.
pub fn capture_file(db: &Path, started_ms: i64) -> Option<PathBuf> {
    Some(
        db.parent()?
            .join("captures")
            .join(format!("{started_ms}.capture")),
    )
}

pub fn helper_args(minutes: u64, out: &Path) -> String {
    format!(
        "deep-capture --minutes {minutes} --out \"{}\"",
        out.display()
    )
}

/// The elevated helper's process, closed on drop.
pub struct Elevated(HANDLE);

// SAFETY: a process handle may be waited on and closed from any thread.
unsafe impl Send for Elevated {}

impl Elevated {
    /// Waits for the helper to exit and returns its exit code.
    pub fn wait(&self) -> Option<u32> {
        // SAFETY: the handle is the live process handle ShellExecuteExW returned.
        if unsafe { WaitForSingleObject(self.0, INFINITE) } != WAIT_OBJECT_0 {
            return None;
        }
        let mut code = 0u32;
        // SAFETY: reads the exit code of the process this handle names into a local.
        unsafe { GetExitCodeProcess(self.0, &mut code) }.ok()?;
        Some(code)
    }
}

impl Drop for Elevated {
    fn drop(&mut self) {
        // SAFETY: closes the handle this value owns, once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Asks Windows to run `exe` elevated; a declined permission prompt is `UiError::Declined`.
pub fn launch_elevated(exe: &Path, args: &str) -> Result<Elevated, UiError> {
    let file: Vec<u16> = exe.as_os_str().encode_wide().chain([0]).collect();
    let params: Vec<u16> = args.encode_utf16().chain([0]).collect();
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: info and the strings it points to live through the call.
    if let Err(err) = unsafe { ShellExecuteExW(&mut info) } {
        if err.code() == ERROR_CANCELLED.to_hresult() {
            return Err(UiError::Declined);
        }
        return Err(UiError::Failed(err.to_string()));
    }
    if info.hProcess.is_invalid() {
        return Err(UiError::Failed("Windows started no helper process".into()));
    }
    Ok(Elevated(info.hProcess))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_file_sits_beside_the_database() {
        let db = Path::new(r"C:\Users\someone\AppData\Local\Vestigium\data\vestigium.db");
        assert_eq!(
            capture_file(db, 1_793_580_000_000).unwrap(),
            PathBuf::from(
                r"C:\Users\someone\AppData\Local\Vestigium\data\captures\1793580000000.capture"
            )
        );
    }

    #[test]
    fn helper_args_quote_the_output() {
        let out = Path::new(r"C:\Users\some one\AppData\Local\Vestigium\data\captures\5.capture");
        assert_eq!(
            helper_args(5, out),
            r#"deep-capture --minutes 5 --out "C:\Users\some one\AppData\Local\Vestigium\data\captures\5.capture""#
        );
    }
}
