use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_CANCELLED, HANDLE, HLOCAL, LocalFree, WAIT_OBJECT_0,
};
use windows::Win32::Security::Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT};
use windows::Win32::Security::{EqualSid, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID};
use windows::Win32::System::Threading::{
    CreateMutexW, GetExitCodeProcess, OpenMutexW, SYNCHRONIZATION_ACCESS_RIGHTS,
    WaitForSingleObject,
};
use windows::Win32::UI::Shell::{
    SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{PCWSTR, w};

use crate::agent::UiError;
use crate::protected::{account_could_change, this_process_is_elevated};

/// Capture lengths the window accepts, in minutes.
pub const CAPTURE_MINUTES: std::ops::RangeInclusive<u64> = 1..=vestigium_core::CAPTURE_MINUTES_MAX;

/// A deep capture under way, as far as the window knows: when it started and when it should end,
/// Unix epoch milliseconds. A capture another window started has no known end, and maybe no start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CaptureRun {
    pub started_ms: Option<i64>,
    pub ends_ms: Option<i64>,
    /// This window started it and can stop it.
    pub own: bool,
}

impl CaptureRun {
    /// A capture this window started.
    pub fn started_here(started_ms: i64, minutes: u64) -> CaptureRun {
        CaptureRun {
            started_ms: Some(started_ms),
            ends_ms: Some(started_ms + minutes as i64 * 60_000),
            own: true,
        }
    }
}

/// What the window found when it looked for the helper's capture lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockState {
    /// A mutex of that name exists and a helper owns it.
    Held,
    /// No mutex of that name, or one that a user process made.
    Free,
    /// The name exists and the window may not read it: it counts as held.
    Unreadable,
}

/// The exit code of a deep capture helper that found another capture running.
pub const CAPTURE_RUNNING: u32 = 4;

/// How a wait for the helper ended, from the window's side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Ended {
    /// The helper exited by itself.
    #[default]
    Done,
    /// The person cancelled; the helper was told to stop, or the window stopped waiting for it.
    Cancelled,
    /// The helper was still running at its ceiling; the window stopped waiting for it.
    TimedOut,
    /// The helper refused to start because another capture was running.
    Refused,
}

/// What the window hears when the helper exits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaptureFinished {
    /// The captures the watcher stored, newest last.
    pub ids: Vec<i64>,
    pub exit_code: Option<u32>,
    pub ended: Ended,
}

/// The exit code of a deep capture that a stop file ended.
pub const CAPTURE_CANCELLED: u32 = 5;
/// How long a deep capture's helper gets to exit after a cancel before the window stops waiting.
pub const CAPTURE_GRACE: Duration = Duration::from_secs(10);
/// The same for a volume check or a disk scan, which end a read or kill chkdsk first.
pub const TEST_GRACE: Duration = Duration::from_secs(30);
/// The extra time a deep capture gets past its length before the window gives up on its helper.
pub const CAPTURE_MARGIN: Duration = Duration::from_secs(120);
/// The longest a health scan or a dump read may take.
pub const SCAN_CEILING: Duration = Duration::from_secs(30 * 60);
/// The longest an elevated batch of tweaks may take.
pub const TWEAK_CEILING: Duration = Duration::from_secs(10 * 60);
/// How often a wait looks at the clock and at a cancel request.
const SLICE_MS: u32 = 250;

/// How a wait for the helper ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waited {
    /// The helper exited; its exit code, when it has one.
    Exited(Option<u32>),
    /// The ceiling passed with the helper still running.
    TimedOut,
    /// A cancel was asked for and the helper did not exit within its grace.
    Abandoned,
}

/// A helper wait as a command's result: the exit code, or the error for a ceiling or a cancel.
pub fn wait_result(
    process: &Elevated,
    ceiling: Duration,
    cancel: &AtomicBool,
) -> Result<Option<u32>, UiError> {
    let cancelled = || cancel.load(Ordering::Acquire);
    match process.wait(Some(ceiling), &cancelled, Duration::ZERO) {
        Waited::Exited(code) => Ok(code),
        Waited::TimedOut => Err(UiError::HelperTimedOut),
        Waited::Abandoned => Err(UiError::Cancelled),
    }
}

/// The exit code and ending a deep capture's wait amounts to. A capture that saved its result
/// (exit code 0) is done even when a cancel came too late; otherwise a cancel, or the helper's own
/// cancelled code, makes it cancelled.
pub fn capture_end(waited: Waited, asked: bool) -> (Option<u32>, Ended) {
    match waited {
        Waited::Exited(Some(0)) => (Some(0), Ended::Done),
        Waited::Exited(Some(CAPTURE_RUNNING)) => (Some(CAPTURE_RUNNING), Ended::Refused),
        Waited::Exited(code) if asked || code == Some(CAPTURE_CANCELLED) => {
            (code, Ended::Cancelled)
        }
        Waited::Exited(code) => (code, Ended::Done),
        Waited::TimedOut => (None, Ended::TimedOut),
        Waited::Abandoned => (None, Ended::Cancelled),
    }
}

/// The same for a test the helper runs, which writes a partial result when stopped: asked to stop
/// means cancelled whatever the exit code.
pub fn test_end(waited: Waited, asked: bool) -> (Option<u32>, Ended) {
    match waited {
        Waited::Exited(code) if asked => (code, Ended::Cancelled),
        Waited::Exited(code) => (code, Ended::Done),
        Waited::TimedOut => (None, Ended::TimedOut),
        Waited::Abandoned => (None, Ended::Cancelled),
    }
}

/// A deep capture the window may stop: the flag its wait watches and the file its helper polls.
#[derive(Clone, Debug)]
pub struct CaptureStop {
    pub asked: Arc<AtomicBool>,
    pub file: PathBuf,
}

impl CaptureStop {
    pub fn new(file: PathBuf) -> CaptureStop {
        CaptureStop {
            asked: Arc::new(AtomicBool::new(false)),
            file,
        }
    }

    /// Tells the helper to stop and the wait to give up after its grace.
    pub fn ask(&self) -> std::io::Result<()> {
        self.asked.store(true, Ordering::Release);
        std::fs::write(&self.file, b"")
    }
}

/// Counts the time since a cancel was first seen.
pub struct CancelClock {
    asked: Option<Instant>,
    grace: Duration,
}

impl CancelClock {
    pub fn new(grace: Duration) -> CancelClock {
        CancelClock { asked: None, grace }
    }

    /// Whether `requested` has held for longer than the grace.
    pub fn expired(&mut self, requested: bool) -> bool {
        if requested {
            self.asked.get_or_insert_with(Instant::now).elapsed() >= self.grace
        } else {
            false
        }
    }
}

/// Waits until `exited_within` (a poll of at most the given milliseconds) reports an exit, the
/// `ceiling` passes, or `cancelled` has held for `grace`; a helper that exits is never reported
/// as given up on.
pub fn wait_for(
    mut exited_within: impl FnMut(u32) -> Option<Option<u32>>,
    ceiling: Option<Duration>,
    cancelled: &dyn Fn() -> bool,
    grace: Duration,
) -> Waited {
    let started = Instant::now();
    let mut clock = CancelClock::new(grace);
    loop {
        if let Some(code) = exited_within(SLICE_MS) {
            return Waited::Exited(code);
        }
        if clock.expired(cancelled()) {
            return Waited::Abandoned;
        }
        if ceiling.is_some_and(|ceiling| started.elapsed() >= ceiling) {
            return Waited::TimedOut;
        }
    }
}

/// What the window gets back when a full scan's helper exits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScanFinished {
    /// The scans the watcher stored; empty while the watcher is stopped.
    pub scans: Vec<i64>,
    pub exit_code: Option<u32>,
}

/// What the window says when it will not start the helper.
pub const HELPER_UNAVAILABLE: &str =
    "the helper is missing, or sits where this account could replace it";

const INSTALLED_HELPER: &str = "Vestigium.Helper.exe";
#[cfg(debug_assertions)]
const DEVELOPMENT_HELPER: &str = "vestigium-helper.exe";

/// The helper next to this window, when this account cannot replace it.
pub fn sibling_helper() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    helper_in(exe.parent()?, this_process_is_elevated())
}

/// The installed helper in `dir` unless `could_change` says this account could replace it; a
/// development build also takes the helper cargo builds. An elevated window has nothing to protect.
fn helper_in(dir: &Path, elevated: bool) -> Option<PathBuf> {
    let installed = dir.join(INSTALLED_HELPER);
    if installed.is_file() && (elevated || !account_could_change(&installed)) {
        return Some(installed);
    }
    #[cfg(debug_assertions)]
    {
        let development = dir.join(DEVELOPMENT_HELPER);
        if development.is_file() {
            return Some(development);
        }
    }
    None
}

/// `<data>\captures\<start ms>.capture`, beside the watcher's database.
pub fn capture_file(db: &Path, started_ms: i64) -> Option<PathBuf> {
    Some(
        db.parent()?
            .join("captures")
            .join(format!("{started_ms}.capture")),
    )
}

/// Access to read a kernel object's owner, and to wait on it.
const READ_CONTROL: u32 = 0x0002_0000;
const SYNCHRONIZE: u32 = 0x0010_0000;

/// Looks for the mutex `name`; one a process of this account owns is a squatter's, not the
/// helper's, and an elevated window cannot tell and takes any mutex as held.
pub fn lock_state(name: &str) -> LockState {
    let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
    // SAFETY: wide is a null-terminated name that outlives the call.
    let handle = match unsafe {
        OpenMutexW(
            SYNCHRONIZATION_ACCESS_RIGHTS(READ_CONTROL | SYNCHRONIZE),
            false,
            PCWSTR(wide.as_ptr()),
        )
    } {
        Ok(handle) => handle,
        Err(err) if err.code() == ERROR_ACCESS_DENIED.to_hresult() => {
            return LockState::Unreadable;
        }
        Err(_) => return LockState::Free,
    };
    let same = (!this_process_is_elevated())
        .then(|| owned_like_a_new_object(handle))
        .flatten();
    // SAFETY: the handle was returned by OpenMutexW and is closed once.
    let _ = unsafe { CloseHandle(handle) };
    lock_from_owner(same, this_process_is_elevated())
}

/// The lock a mutex that exists stands for, given whether this account would own it (`None` when
/// that could not be read) and whether this window is elevated.
pub fn lock_from_owner(owned_like_this_account: Option<bool>, elevated: bool) -> LockState {
    match owned_like_this_account {
        _ if elevated => LockState::Held,
        Some(true) => LockState::Free,
        Some(false) => LockState::Held,
        None => LockState::Unreadable,
    }
}

/// Whether `handle`'s object has the owner a mutex this process creates gets; `None` when an
/// owner could not be read.
fn owned_like_a_new_object(handle: HANDLE) -> Option<bool> {
    let own: Vec<u16> = format!(r"Local\Vestigium.Owner.{}", std::process::id())
        .encode_utf16()
        .chain([0])
        .collect();
    // SAFETY: own is a null-terminated name that outlives the call; the handle is closed below.
    let mine = unsafe { CreateMutexW(None, false, PCWSTR(own.as_ptr())) }.ok()?;
    let theirs = owner(handle);
    let ours = owner(mine);
    // SAFETY: the handle was returned by CreateMutexW and is closed once.
    let _ = unsafe { CloseHandle(mine) };
    let same = match (&theirs, &ours) {
        // SAFETY: both sids live in descriptors that are freed below.
        (Some((a, _)), Some((b, _))) => Some(unsafe { EqualSid(*a, *b) }.is_ok()),
        _ => None,
    };
    for (_, descriptor) in theirs.into_iter().chain(ours) {
        // SAFETY: the descriptor was allocated by GetSecurityInfo for LocalFree.
        let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    }
    same
}

/// The owner sid of a kernel object and the descriptor that holds it.
fn owner(handle: HANDLE) -> Option<(PSID, PSECURITY_DESCRIPTOR)> {
    let mut sid = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: both out pointers are locals; the caller frees the descriptor.
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut sid),
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    };
    (status.is_ok() && !sid.0.is_null()).then_some((sid, descriptor))
}

/// The names in `<data>\captures`, beside the watcher's database.
fn capture_names(db: &Path) -> Vec<String> {
    let Some(folder) = db.parent().map(|data| data.join("captures")) else {
        return Vec::new();
    };
    std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// The capture another window, or another account, started and that still runs; `None` when the
/// helper's lock is free.
pub fn capture_elsewhere(db: Option<&Path>, now_ms: i64) -> Option<CaptureRun> {
    let lock = lock_state(vestigium_collect::DEEP_CAPTURE_LOCK);
    let names = db.map(capture_names).unwrap_or_default();
    running_elsewhere(
        lock,
        started_from_temp(names.iter().map(String::as_str), now_ms),
    )
}

/// The start stamp of a result a helper is still writing: `<start ms>.capture.tmp`.
pub fn temp_stamp(name: &str) -> Option<i64> {
    let digits = name.strip_suffix(".capture.tmp")?;
    if digits.is_empty() || digits.len() > 15 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The newest start stamp among `names` that a capture running now could have: not in the future
/// and not older than the longest capture plus the wait's margin.
pub fn started_from_temp<'a>(names: impl IntoIterator<Item = &'a str>, now_ms: i64) -> Option<i64> {
    let longest =
        vestigium_core::CAPTURE_MINUTES_MAX as i64 * 60_000 + CAPTURE_MARGIN.as_millis() as i64;
    names
        .into_iter()
        .filter_map(temp_stamp)
        .filter(|&stamp| stamp <= now_ms + 60_000 && now_ms - stamp <= longest)
        .max()
}

/// The capture another window started, from what the lock says and the newest temporary result:
/// nothing while the lock is free; the start is known only when a temporary result names it.
pub fn running_elsewhere(lock: LockState, started_ms: Option<i64>) -> Option<CaptureRun> {
    match lock {
        LockState::Free => None,
        LockState::Held | LockState::Unreadable => Some(CaptureRun {
            started_ms,
            ends_ms: None,
            own: false,
        }),
    }
}

/// `<data>\scans\<start ms>.scan`, beside the watcher's database.
pub fn scan_file(db: &Path, started_ms: i64) -> Option<PathBuf> {
    Some(
        db.parent()?
            .join("scans")
            .join(format!("{started_ms}.scan")),
    )
}

/// `<data>\dumps\<start ms>.<extension>`, beside the watcher's database.
pub fn dumps_file(db: &Path, started_ms: i64, extension: &str) -> Option<PathBuf> {
    Some(
        db.parent()?
            .join("dumps")
            .join(format!("{started_ms}.{extension}")),
    )
}

/// ` --user-sid <SID>` for the account that asked, so the helper reads that account's dumps.
fn user_sid_arg(sid: Option<&str>) -> String {
    sid.map(|sid| format!(" --user-sid {sid}"))
        .unwrap_or_default()
}

pub fn dump_scan_args(out: &Path, user_sid: Option<&str>) -> String {
    format!(
        "dump-scan --out \"{}\"{}",
        out.display(),
        user_sid_arg(user_sid)
    )
}

pub fn delete_dumps_args(list: &Path, user_sid: Option<&str>) -> String {
    format!(
        "delete-dumps --list \"{}\"{}",
        list.display(),
        user_sid_arg(user_sid)
    )
}

/// What the window gets back when the helper that read the dumps exits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DumpScanFinished {
    /// The dumps the watcher stored or refreshed; empty while the watcher is stopped.
    pub dumps: Vec<i64>,
    pub exit_code: Option<u32>,
}

/// What the window gets back when the helper that deleted dumps exits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DeleteFinished {
    /// How many dumps the watcher marked as deleted.
    pub deleted: u32,
    /// How many paths the helper refused to delete, and how many it could not delete.
    pub refused: u32,
    pub failed: u32,
    pub exit_code: Option<u32>,
}

pub fn scan_args(out: &Path) -> String {
    format!("health-scan --out \"{}\"", out.display())
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
    /// The exit code when the helper exits within `ms`; `None` while it still runs.
    pub fn exited_within(&self, ms: u32) -> Option<Option<u32>> {
        // SAFETY: the handle is the live process handle ShellExecuteExW returned.
        if unsafe { WaitForSingleObject(self.0, ms) } != WAIT_OBJECT_0 {
            return None;
        }
        let mut code = 0u32;
        // SAFETY: reads the exit code of the process this handle names into a local.
        Some(
            unsafe { GetExitCodeProcess(self.0, &mut code) }
                .ok()
                .map(|()| code),
        )
    }

    /// Waits for the helper to exit, to the `ceiling` if there is one, or until `cancelled` has
    /// held for `grace`.
    pub fn wait(
        &self,
        ceiling: Option<Duration>,
        cancelled: &dyn Fn() -> bool,
        grace: Duration,
    ) -> Waited {
        wait_for(|ms| self.exited_within(ms), ceiling, cancelled, grace)
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

    fn helper_dir(names: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for name in names {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        dir
    }

    #[test]
    fn a_capture_that_saved_its_result_is_done_whatever_was_asked() {
        assert_eq!(
            capture_end(Waited::Exited(Some(0)), true),
            (Some(0), Ended::Done)
        );
        assert_eq!(
            capture_end(Waited::Exited(Some(CAPTURE_CANCELLED)), false),
            (Some(CAPTURE_CANCELLED), Ended::Cancelled)
        );
        assert_eq!(
            capture_end(Waited::Exited(Some(1)), true),
            (Some(1), Ended::Cancelled)
        );
        assert_eq!(
            capture_end(Waited::Exited(Some(1)), false),
            (Some(1), Ended::Done)
        );
        assert_eq!(
            capture_end(Waited::TimedOut, false),
            (None, Ended::TimedOut)
        );
        assert_eq!(
            capture_end(Waited::Abandoned, true),
            (None, Ended::Cancelled)
        );
    }

    #[test]
    fn a_helper_that_found_a_capture_running_ends_as_refused() {
        assert_eq!(
            capture_end(Waited::Exited(Some(CAPTURE_RUNNING)), false),
            (Some(CAPTURE_RUNNING), Ended::Refused)
        );
        assert_eq!(
            capture_end(Waited::Exited(Some(CAPTURE_RUNNING)), true),
            (Some(CAPTURE_RUNNING), Ended::Refused)
        );
    }

    #[test]
    fn only_a_capture_result_still_being_written_names_a_start() {
        assert_eq!(
            temp_stamp("1793580000000.capture.tmp"),
            Some(1_793_580_000_000)
        );
        assert_eq!(temp_stamp("1793580000000.capture"), None);
        assert_eq!(temp_stamp("1793580000000.scan.tmp"), None);
        assert_eq!(temp_stamp("x.capture.tmp"), None);
        assert_eq!(temp_stamp(".capture.tmp"), None);
        assert_eq!(temp_stamp("-5.capture.tmp"), None);
        assert_eq!(temp_stamp("12345678901234567890.capture.tmp"), None);
    }

    #[test]
    fn the_newest_plausible_temporary_result_gives_the_start() {
        let now = 1_793_580_000_000;
        let names = [
            "1793579000000.capture.tmp",
            "1793579900000.capture.tmp",
            "1793579900000.capture",
            "notes.txt",
        ];
        assert_eq!(started_from_temp(names, now), Some(1_793_579_900_000));
        assert_eq!(started_from_temp(["notes.txt"], now), None);
    }

    #[test]
    fn a_temporary_result_from_the_future_or_older_than_any_capture_is_ignored() {
        let now = 1_793_580_000_000;
        let day = 24 * 3_600_000;
        assert_eq!(
            started_from_temp([format!("{}.capture.tmp", now + 3_600_000).as_str()], now),
            None
        );
        assert_eq!(
            started_from_temp([format!("{}.capture.tmp", now - 2 * day).as_str()], now),
            None
        );
        assert_eq!(
            started_from_temp(
                [format!("{}.capture.tmp", now - day + 60_000).as_str()],
                now
            ),
            Some(now - day + 60_000)
        );
    }

    #[test]
    fn a_free_lock_means_no_capture_whatever_files_are_left() {
        assert_eq!(running_elsewhere(LockState::Free, Some(5)), None);
        assert_eq!(running_elsewhere(LockState::Free, None), None);
    }

    #[test]
    fn a_held_or_unreadable_lock_means_a_capture_with_an_unknown_end() {
        for lock in [LockState::Held, LockState::Unreadable] {
            assert_eq!(
                running_elsewhere(lock, None),
                Some(CaptureRun {
                    started_ms: None,
                    ends_ms: None,
                    own: false
                })
            );
            assert_eq!(
                running_elsewhere(lock, Some(9)),
                Some(CaptureRun {
                    started_ms: Some(9),
                    ends_ms: None,
                    own: false
                })
            );
        }
    }

    #[test]
    fn a_capture_started_here_knows_both_ends() {
        assert_eq!(
            CaptureRun::started_here(1_000, 5),
            CaptureRun {
                started_ms: Some(1_000),
                ends_ms: Some(301_000),
                own: true
            }
        );
    }

    fn lock_name(tag: &str) -> Vec<u16> {
        format!(r"Local\Vestigium.Test.{}.{tag}", std::process::id())
            .encode_utf16()
            .chain([0])
            .collect()
    }

    #[test]
    fn a_name_nobody_holds_is_a_free_lock() {
        let name = format!(r"Local\Vestigium.Test.{}.absent", std::process::id());
        assert_eq!(lock_state(&name), LockState::Free);
    }

    #[test]
    fn a_mutex_a_user_process_made_is_not_the_helpers_lock() {
        if this_process_is_elevated() {
            return;
        }
        let wide = lock_name("squat");
        // SAFETY: wide is a null-terminated name that outlives the call; the handle is closed below.
        let mutex = unsafe {
            windows::Win32::System::Threading::CreateMutexW(None, false, PCWSTR(wide.as_ptr()))
        }
        .unwrap();
        let name = String::from_utf16_lossy(&wide[..wide.len() - 1]);
        assert_eq!(lock_state(&name), LockState::Free);
        // SAFETY: the handle was returned by CreateMutexW and is closed once.
        let _ = unsafe { CloseHandle(mutex) };
    }

    #[test]
    fn a_mutex_owned_by_someone_else_is_held_and_one_this_account_owns_is_not() {
        assert_eq!(lock_from_owner(Some(false), false), LockState::Held);
        assert_eq!(lock_from_owner(Some(true), false), LockState::Free);
        assert_eq!(lock_from_owner(None, false), LockState::Unreadable);
    }

    #[test]
    fn an_elevated_window_takes_any_existing_mutex_as_held() {
        for owned in [Some(true), Some(false), None] {
            assert_eq!(lock_from_owner(owned, true), LockState::Held);
        }
    }

    /// Start a deep capture helper elevated (the installed one, for a few minutes), then run this
    /// test unelevated: `cargo test -p vestigium-ui -- --ignored a_running_helper`.
    #[test]
    #[ignore = "needs a deep capture helper running elevated while this runs unelevated"]
    fn a_running_helper_holds_the_lock_the_window_sees() {
        assert_eq!(
            lock_state(vestigium_collect::DEEP_CAPTURE_LOCK),
            LockState::Held
        );
    }

    #[test]
    fn a_test_asked_to_stop_is_cancelled_even_with_a_partial_result() {
        assert_eq!(
            test_end(Waited::Exited(Some(0)), true),
            (Some(0), Ended::Cancelled)
        );
        assert_eq!(
            test_end(Waited::Exited(Some(0)), false),
            (Some(0), Ended::Done)
        );
        assert_eq!(test_end(Waited::Abandoned, true), (None, Ended::Cancelled));
    }

    #[test]
    fn asking_a_capture_to_stop_writes_its_stop_file() {
        let dir = tempfile::tempdir().unwrap();
        let stop = CaptureStop::new(dir.path().join("5.stop"));
        stop.ask().unwrap();
        assert!(stop.asked.load(Ordering::Acquire));
        assert!(dir.path().join("5.stop").is_file());
    }

    #[test]
    fn a_helper_that_never_exits_is_given_up_at_the_ceiling() {
        let started = Instant::now();
        let waited = wait_for(
            |ms| {
                std::thread::sleep(Duration::from_millis(u64::from(ms)));
                None
            },
            Some(Duration::from_millis(400)),
            &|| false,
            Duration::from_secs(60),
        );
        assert_eq!(waited, Waited::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_helper_that_exits_returns_its_code() {
        let mut polls = 0;
        let waited = wait_for(
            |_| {
                polls += 1;
                (polls == 3).then_some(Some(7))
            },
            Some(Duration::from_secs(60)),
            &|| false,
            Duration::from_secs(60),
        );
        assert_eq!(waited, Waited::Exited(Some(7)));
    }

    #[test]
    fn a_cancel_is_given_its_grace_and_then_the_wait_ends() {
        let started = Instant::now();
        let waited = wait_for(
            |ms| {
                std::thread::sleep(Duration::from_millis(u64::from(ms)));
                None
            },
            None,
            &|| true,
            Duration::from_millis(500),
        );
        assert_eq!(waited, Waited::Abandoned);
        assert!(started.elapsed() >= Duration::from_millis(500));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_helper_that_exits_within_the_grace_of_a_cancel_is_not_abandoned() {
        let mut polls = 0;
        let waited = wait_for(
            |ms| {
                polls += 1;
                std::thread::sleep(Duration::from_millis(u64::from(ms)));
                (polls == 2).then_some(Some(5))
            },
            None,
            &|| true,
            Duration::from_secs(30),
        );
        assert_eq!(waited, Waited::Exited(Some(5)));
    }

    #[test]
    fn a_wait_without_a_ceiling_or_a_cancel_goes_on() {
        let mut polls = 0;
        let waited = wait_for(
            |_| {
                polls += 1;
                (polls == 20).then_some(Some(0))
            },
            None,
            &|| false,
            Duration::ZERO,
        );
        assert_eq!(waited, Waited::Exited(Some(0)));
    }

    #[test]
    fn an_installed_helper_in_a_folder_the_account_can_write_is_refused() {
        let dir = helper_dir(&[INSTALLED_HELPER]);
        assert_eq!(helper_in(dir.path(), false), None);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn the_development_helper_still_resolves() {
        let dir = helper_dir(&[INSTALLED_HELPER, DEVELOPMENT_HELPER]);
        assert_eq!(
            helper_in(dir.path(), false),
            Some(dir.path().join(DEVELOPMENT_HELPER))
        );
    }

    #[test]
    fn an_elevated_window_takes_the_installed_helper_anywhere() {
        let dir = helper_dir(&[INSTALLED_HELPER]);
        assert_eq!(
            helper_in(dir.path(), true),
            Some(dir.path().join(INSTALLED_HELPER))
        );
    }

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
    fn scan_file_and_args_sit_beside_the_database() {
        let db = Path::new(r"C:\Users\some one\AppData\Local\Vestigium\data\vestigium.db");
        let out = scan_file(db, 7).unwrap();
        assert_eq!(
            out,
            PathBuf::from(r"C:\Users\some one\AppData\Local\Vestigium\data\scans\7.scan")
        );
        assert_eq!(
            scan_args(&out),
            r#"health-scan --out "C:\Users\some one\AppData\Local\Vestigium\data\scans\7.scan""#
        );
    }

    #[test]
    fn dump_files_and_args_sit_beside_the_database() {
        let db = Path::new(r"C:\Users\some one\AppData\Local\Vestigium\data\vestigium.db");
        let out = dumps_file(db, 7, "dumps").unwrap();
        assert_eq!(
            out,
            PathBuf::from(r"C:\Users\some one\AppData\Local\Vestigium\data\dumps\7.dumps")
        );
        assert_eq!(
            dump_scan_args(&out, None),
            r#"dump-scan --out "C:\Users\some one\AppData\Local\Vestigium\data\dumps\7.dumps""#
        );
        let list = dumps_file(db, 8, "delete").unwrap();
        assert_eq!(
            delete_dumps_args(&list, None),
            r#"delete-dumps --list "C:\Users\some one\AppData\Local\Vestigium\data\dumps\8.delete""#
        );
    }

    #[test]
    fn dump_args_name_the_user_who_asked() {
        let sid = "S-1-5-21-1004336348-1177238915-682003330-1001";
        assert_eq!(
            dump_scan_args(Path::new(r"C:\d\7.dumps"), Some(sid)),
            format!(r#"dump-scan --out "C:\d\7.dumps" --user-sid {sid}"#)
        );
        assert_eq!(
            delete_dumps_args(Path::new(r"C:\d\8.delete"), Some(sid)),
            format!(r#"delete-dumps --list "C:\d\8.delete" --user-sid {sid}"#)
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
