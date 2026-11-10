//! The deep-capture lock: a named mutex that exists while one capture runs.

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HANDLE, HLOCAL, LocalFree,
    SetLastError,
};
use windows::Win32::Security::Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT};
use windows::Win32::Security::{EqualSid, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

/// Holds the lock until it is dropped or the process ends.
#[derive(Debug)]
pub struct CaptureLock(HANDLE);

impl Drop for CaptureLock {
    fn drop(&mut self) {
        // SAFETY: the handle was returned by CreateMutexW and is closed once.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Another helper holds the lock.
#[derive(Debug, PartialEq, Eq)]
pub struct Running;

/// Takes the lock called `name`: the guard to hold while the capture runs, or `Running`. Any
/// process may create a global name, so a name whose owner is not the owner this helper's own
/// objects get is a squatter's; it holds nothing back, and the result is no guard.
pub fn take(name: &str) -> Result<Option<CaptureLock>, Running> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: only clears this thread's last error.
    unsafe { SetLastError(ERROR_SUCCESS) };
    // SAFETY: wide is a null-terminated name that outlives the call.
    let handle = match unsafe { CreateMutexW(None, false, PCWSTR(wide.as_ptr())) } {
        Ok(handle) => handle,
        Err(_) => return Ok(None),
    };
    // SAFETY: read straight after the call that set it.
    if unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
        return Ok(Some(CaptureLock(handle)));
    }
    let ours = same_owner_as_a_new_object(handle);
    // SAFETY: the handle was returned by CreateMutexW and is closed once.
    unsafe {
        let _ = CloseHandle(handle);
    }
    if ours { Err(Running) } else { Ok(None) }
}

/// Whether `handle`'s object has the owner a mutex this process creates gets.
fn same_owner_as_a_new_object(handle: HANDLE) -> bool {
    // An unnamed object has no owner to read, so a named one stands in.
    let own: Vec<u16> = format!(r"Local\Vestigium.Owner.{}", std::process::id())
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: own is a null-terminated name that outlives the call; the handle is closed below.
    let Ok(mine) = (unsafe { CreateMutexW(None, false, PCWSTR(own.as_ptr())) }) else {
        return false;
    };
    let same = match (owner(handle), owner(mine)) {
        (Some((a, a_sd)), Some((b, b_sd))) => {
            // SAFETY: both sids live in security descriptors that are freed after the comparison.
            let same = unsafe { EqualSid(a, b) }.is_ok();
            free(a_sd);
            free(b_sd);
            same
        }
        (a, b) => {
            for (_, sd) in a.into_iter().chain(b) {
                free(sd);
            }
            false
        }
    };
    // SAFETY: the handle was returned by CreateMutexW and is closed once.
    unsafe {
        let _ = CloseHandle(mine);
    }
    same
}

/// The owner sid of a kernel object and the descriptor that holds it, which `free` releases.
fn owner(handle: HANDLE) -> Option<(PSID, PSECURITY_DESCRIPTOR)> {
    let mut sid = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: both out pointers are locals; the descriptor is released by `free`.
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

fn free(descriptor: PSECURITY_DESCRIPTOR) {
    // SAFETY: the descriptor was allocated by GetSecurityInfo for LocalFree.
    unsafe {
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(tag: &str) -> String {
        format!(r"Local\Vestigium.Test.{}.{tag}", std::process::id())
    }

    #[test]
    fn a_second_take_of_the_same_name_finds_it_running() {
        let name = name("twice");
        let first = take(&name);
        assert!(matches!(first, Ok(Some(_))));
        assert_eq!(take(&name).err(), Some(Running));
        drop(first);
        assert!(matches!(take(&name), Ok(Some(_))));
    }

    #[test]
    fn different_names_do_not_hold_each_other() {
        let a = take(&name("a"));
        let b = take(&name("b"));
        assert!(matches!(a, Ok(Some(_))));
        assert!(matches!(b, Ok(Some(_))));
    }
}
