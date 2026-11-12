//! Whether the account running the window could replace a file or reroute the folders it sits in.

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    AccessCheck, DACL_SECURITY_INFORMATION, DuplicateToken, GENERIC_MAPPING,
    GROUP_SECURITY_INFORMATION, GetFileSecurityW, GetTokenInformation, OWNER_SECURITY_INFORMATION,
    PRIVILEGE_SET, PSECURITY_DESCRIPTOR, SecurityImpersonation, TOKEN_DUPLICATE, TOKEN_ELEVATION,
    TOKEN_QUERY, TokenElevation,
};
use windows::Win32::Storage::FileSystem::{
    DELETE, FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_ALL_ACCESS, FILE_APPEND_DATA,
    FILE_DELETE_CHILD, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_WRITE_DATA, WRITE_DAC, WRITE_OWNER,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::PCWSTR;

/// `MAXIMUM_ALLOWED`: ask for every right the account holds instead of testing one set.
const MAXIMUM_ALLOWED: u32 = 0x0200_0000;

/// Rights that change or replace the file itself.
const FILE_RIGHTS: u32 =
    FILE_WRITE_DATA.0 | FILE_APPEND_DATA.0 | DELETE.0 | WRITE_DAC.0 | WRITE_OWNER.0;
/// Rights that remove or rename the folder, or the entry below it that leads to the file.
const PATH_RIGHTS: u32 = FILE_DELETE_CHILD.0 | DELETE.0 | WRITE_DAC.0 | WRITE_OWNER.0;
/// Rights that put a new file or folder beside the program.
const NEIGHBOUR_RIGHTS: u32 = FILE_ADD_FILE.0 | FILE_ADD_SUBDIRECTORY.0;

/// True when this account could change `path`, or any folder on the way to it, so a program
/// running as the account could swap the file. An unreadable access list counts as writable.
pub fn account_could_change(path: &Path) -> bool {
    let Ok(real) = std::fs::canonicalize(path) else {
        return true;
    };
    let Some(token) = Impersonation::of_this_process() else {
        return true;
    };
    let granted = |target: &Path| token.granted(target);
    let Some(file) = granted(&real) else {
        return true;
    };
    if file & FILE_RIGHTS != 0 {
        return true;
    }
    for (depth, folder) in real.ancestors().skip(1).enumerate() {
        let Some(rights) = granted(folder) else {
            return true;
        };
        let wanted = if depth == 0 {
            PATH_RIGHTS | NEIGHBOUR_RIGHTS
        } else {
            PATH_RIGHTS
        };
        if rights & wanted != 0 {
            return true;
        }
    }
    false
}

/// True when this process already runs with an administrator's full token.
pub fn this_process_is_elevated() -> bool {
    let mut raw = HANDLE::default();
    // SAFETY: opens a query handle on the current process's token, closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw) }.is_err() {
        return false;
    }
    let mut answer = TOKEN_ELEVATION::default();
    let mut written = 0u32;
    // SAFETY: the buffer is a TOKEN_ELEVATION of the size passed.
    let read = unsafe {
        GetTokenInformation(
            raw,
            TokenElevation,
            Some((&mut answer as *mut TOKEN_ELEVATION).cast::<c_void>()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut written,
        )
    };
    // SAFETY: closes the handle opened above, once.
    let _ = unsafe { CloseHandle(raw) };
    read.is_ok() && answer.TokenIsElevated != 0
}

/// An impersonation copy of this process's token, closed on drop.
struct Impersonation(HANDLE);

impl Impersonation {
    fn of_this_process() -> Option<Impersonation> {
        let mut primary = HANDLE::default();
        // SAFETY: opens a duplicate-and-query handle on the current process's token.
        unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_DUPLICATE | TOKEN_QUERY,
                &mut primary,
            )
        }
        .ok()?;
        let mut copy = HANDLE::default();
        // SAFETY: duplicates the token just opened into `copy`.
        let made = unsafe { DuplicateToken(primary, SecurityImpersonation, &mut copy) };
        // SAFETY: closes the handle opened above, once.
        let _ = unsafe { CloseHandle(primary) };
        made.ok()?;
        Some(Impersonation(copy))
    }

    /// Every right this account holds on `target`, or `None` when the access list cannot be read.
    fn granted(&self, target: &Path) -> Option<u32> {
        let name: Vec<u16> = target.as_os_str().encode_wide().chain([0]).collect();
        let parts =
            (OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION).0;
        let mut needed = 0u32;
        // SAFETY: a size query with no buffer.
        let _ = unsafe { GetFileSecurityW(PCWSTR(name.as_ptr()), parts, None, 0, &mut needed) };
        if needed == 0 {
            return None;
        }
        let mut descriptor = vec![0u8; needed as usize];
        // SAFETY: the buffer holds `needed` bytes.
        let read = unsafe {
            GetFileSecurityW(
                PCWSTR(name.as_ptr()),
                parts,
                Some(PSECURITY_DESCRIPTOR(descriptor.as_mut_ptr().cast())),
                needed,
                &mut needed,
            )
        };
        if !read.as_bool() {
            return None;
        }
        let mapping = GENERIC_MAPPING {
            GenericRead: FILE_GENERIC_READ.0,
            GenericWrite: FILE_GENERIC_WRITE.0,
            GenericExecute: FILE_GENERIC_EXECUTE.0,
            GenericAll: FILE_ALL_ACCESS.0,
        };
        let mut privileges = PRIVILEGE_SET::default();
        let mut privileges_len = size_of::<PRIVILEGE_SET>() as u32;
        let mut granted = 0u32;
        let mut status = windows::core::BOOL(0);
        // SAFETY: the descriptor, the mapping and every out parameter live through the call.
        unsafe {
            AccessCheck(
                PSECURITY_DESCRIPTOR(descriptor.as_mut_ptr().cast()),
                self.0,
                MAXIMUM_ALLOWED,
                &mapping,
                Some(&mut privileges),
                &mut privileges_len,
                &mut granted,
                &mut status,
            )
        }
        .ok()?;
        // With MAXIMUM_ALLOWED a refusal still reports success and grants nothing.
        Some(if status.as_bool() { granted } else { 0 })
    }
}

impl Drop for Impersonation {
    fn drop(&mut self) {
        // SAFETY: closes the handle this value owns, once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_in_a_temporary_folder_can_be_changed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Vestigium.Helper.exe");
        std::fs::write(&file, b"x").unwrap();
        assert!(account_could_change(&file));
    }

    #[test]
    fn a_missing_file_counts_as_changeable() {
        assert!(account_could_change(Path::new(r"C:\Nowhere\none.exe")));
    }

    #[test]
    fn a_system_program_cannot_be_changed_by_a_standard_token() {
        if this_process_is_elevated() {
            return;
        }
        let windir = std::env::var_os("SystemRoot").expect("SystemRoot is set");
        let program = Path::new(&windir).join("System32").join("cmd.exe");
        assert!(!account_could_change(&program));
    }
}
