use std::ffi::c_void;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::{HSTRING, PWSTR};

use crate::IpcError;

/// Security descriptor that grants full access to one SID and nobody else.
pub fn user_only_sddl(sid: &str) -> String {
    format!("D:P(A;;GA;;;{sid})")
}

/// String form of the SID of the user running this process.
pub fn current_user_sid() -> Result<String, IpcError> {
    let mut token = HANDLE::default();
    // SAFETY: the pseudo handle of the current process is always valid; `token` receives a new handle.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)? };
    let sid = token_user_sid(token);
    // SAFETY: `token` was opened above and is closed exactly once.
    unsafe { CloseHandle(token)? };
    sid
}

fn token_user_sid(token: HANDLE) -> Result<String, IpcError> {
    let mut len = 0u32;
    // SAFETY: a size query with no buffer; the expected failure only reports the needed length.
    let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut len) };
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    // SAFETY: `buf` is at least `len` bytes and 8-byte aligned, enough for TOKEN_USER.
    unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast::<c_void>()),
            len,
            &mut len,
        )?
    };
    // SAFETY: GetTokenInformation filled `buf` with a TOKEN_USER whose SID points inside it.
    let sid = unsafe { (*buf.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut text = PWSTR::null();
    // SAFETY: `sid` is valid while `buf` lives; the string is freed with LocalFree below.
    unsafe { ConvertSidToStringSidW(sid, &mut text)? };
    // SAFETY: `text` is a NUL-terminated string allocated by the call above.
    let result = String::from_utf16_lossy(unsafe { text.as_wide() });
    // SAFETY: frees the LocalAlloc'd string exactly once.
    unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };
    Ok(result)
}

/// A self-relative security descriptor parsed from SDDL, freed on drop.
pub(crate) struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    pub(crate) fn from_sddl(sddl: &str) -> Result<SecurityDescriptor, IpcError> {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        // SAFETY: `sddl` outlives the call; `sd` receives a LocalAlloc'd descriptor owned by Self.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                &HSTRING::from(sddl),
                SDDL_REVISION_1,
                &mut sd,
                None,
            )?
        };
        Ok(SecurityDescriptor(sd))
    }

    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.0.0
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: the descriptor came from LocalAlloc and is freed exactly once.
        unsafe { LocalFree(Some(HLOCAL(self.0.0))) };
    }
}
