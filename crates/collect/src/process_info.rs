//! What a program's page shows about one process: its file, who runs it and who signed it.

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use vestigium_core::{RunAs, Signature};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, GENERIC_READ, HANDLE, HWND,
};
use windows::Win32::Security::Cryptography::Catalog::{
    CATALOG_INFO, CryptCATAdminAcquireContext2, CryptCATAdminCalcHashFromFileHandle2,
    CryptCATAdminEnumCatalogFromHash, CryptCATAdminReleaseCatalogContext,
    CryptCATAdminReleaseContext, CryptCATCatalogInfoFromContext,
};
use windows::Win32::Security::Cryptography::{
    BCRYPT_SHA256_ALGORITHM, CERT_NAME_SIMPLE_DISPLAY_TYPE, CertGetNameStringW,
};
use windows::Win32::Security::WinTrust::{
    WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_CATALOG_INFO, WINTRUST_DATA, WINTRUST_DATA_0,
    WINTRUST_FILE_INFO, WTD_CACHE_ONLY_URL_RETRIEVAL, WTD_CHOICE_CATALOG, WTD_CHOICE_FILE,
    WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData, WinVerifyTrust,
};
use windows::Win32::Security::{
    EqualSid, GetTokenInformation, IsWellKnownSid, PSID, TOKEN_ELEVATION, TOKEN_INFORMATION_CLASS,
    TOKEN_QUERY, TOKEN_USER, TokenElevation, TokenUser, WELL_KNOWN_SID_TYPE, WinLocalServiceSid,
    WinLocalSystemSid, WinNetworkServiceSid,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_SEQUENTIAL_SCAN, FILE_SHARE_DELETE, FILE_SHARE_READ, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::{PCWSTR, PWSTR};

/// WinVerifyTrust's answer for a file that carries no signature of its own.
const TRUST_E_NOSIGNATURE: i32 = 0x800B_0100_u32 as i32;
/// WinVerifyTrust's answers for a file it cannot hold a signature in.
const TRUST_E_SUBJECT_FORM_UNKNOWN: i32 = 0x800B_0003_u32 as i32;
const TRUST_E_PROVIDER_UNKNOWN: i32 = 0x800B_0001_u32 as i32;

/// A handle closed when it goes out of scope.
struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by this module and is closed exactly once, here.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain([0]).collect()
}

fn open_process(pid: u32) -> Option<Owned> {
    // SAFETY: asks for query rights only; a pid that is gone or closed just fails.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) };
    process.ok().map(Owned)
}

/// Characters of the first buffer for a process path, and of the one retried when it is too small.
const PATH_UNITS: usize = 1024;
const PATH_UNITS_MAX: usize = 32768;

/// The full path of the program a process runs; `None` when Windows does not let the user ask.
pub fn image_path(pid: u32) -> Option<String> {
    let process = open_process(pid)?;
    let mut size = PATH_UNITS;
    loop {
        let mut buffer = vec![0u16; size];
        let mut len = buffer.len() as u32;
        // SAFETY: writes at most `len` characters into `buffer` and sets `len` to what it wrote.
        let read = unsafe {
            QueryFullProcessImageNameW(
                process.0,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut len,
            )
        };
        match read {
            Ok(()) => return Some(String::from_utf16_lossy(&buffer[..len as usize])),
            Err(err)
                if size < PATH_UNITS_MAX
                    && err.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() =>
            {
                size = PATH_UNITS_MAX;
            }
            Err(_) => return None,
        }
    }
}

/// Reads one block of token information of at least the size of `T`.
fn token_info<T>(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> Option<Vec<u8>> {
    let mut needed = 0u32;
    // SAFETY: a call with no buffer only reports the size the answer needs.
    let _ = unsafe { GetTokenInformation(token, class, None, 0, &mut needed) };
    let mut buffer = vec![0u8; (needed as usize).max(size_of::<T>())];
    // SAFETY: the buffer is as long as the length passed.
    let read = unsafe {
        GetTokenInformation(
            token,
            class,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            buffer.len() as u32,
            &mut needed,
        )
    };
    read.ok()?;
    Some(buffer)
}

fn token_of(process: HANDLE) -> Option<Owned> {
    let mut token = HANDLE::default();
    // SAFETY: asks for query rights on a process handle that is open.
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.ok()?;
    Some(Owned(token))
}

/// The user SID inside a `TokenUser` answer.
fn user_sid(answer: &[u8]) -> PSID {
    // SAFETY: `answer` holds a TOKEN_USER, whose SID points into the same buffer.
    unsafe { (*answer.as_ptr().cast::<TOKEN_USER>()).User.Sid }
}

/// Who a process runs as; `Unknown` when its token is closed to the user.
pub fn run_as(pid: u32) -> RunAs {
    let Some(process) = open_process(pid) else {
        return RunAs::Unknown;
    };
    let Some(token) = token_of(process.0) else {
        return RunAs::Unknown;
    };
    let Some(user) = token_info::<TOKEN_USER>(token.0, TokenUser) else {
        return RunAs::Unknown;
    };
    let sid = user_sid(&user);
    // SAFETY: `sid` points into `user`, alive for these calls.
    let is = |kind: WELL_KNOWN_SID_TYPE| unsafe { IsWellKnownSid(sid, kind) }.as_bool();
    if is(WinLocalSystemSid) {
        return RunAs::System;
    }
    if is(WinLocalServiceSid) {
        return RunAs::LocalService;
    }
    if is(WinNetworkServiceSid) {
        return RunAs::NetworkService;
    }
    // SAFETY: the pseudo handle of this process needs no closing and is always valid.
    let own = token_of(unsafe { GetCurrentProcess() });
    let Some(mine) = own.and_then(|own| token_info::<TOKEN_USER>(own.0, TokenUser)) else {
        return RunAs::Unknown;
    };
    // SAFETY: both SIDs point into buffers alive for the call.
    if unsafe { EqualSid(sid, user_sid(&mine)) }.is_err() {
        return RunAs::OtherUser;
    }
    let elevated = token_info::<TOKEN_ELEVATION>(token.0, TokenElevation).is_some_and(|answer| {
        // SAFETY: `answer` holds a TOKEN_ELEVATION.
        unsafe { (*answer.as_ptr().cast::<TOKEN_ELEVATION>()).TokenIsElevated != 0 }
    });
    RunAs::CurrentUser { elevated }
}

/// Runs one WinVerifyTrust check, offline, and returns its result and the signer's name.
fn verify(data: &mut WINTRUST_DATA) -> (i32, Option<String>) {
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    data.cbStruct = size_of::<WINTRUST_DATA>() as u32;
    data.dwUIChoice = WTD_UI_NONE;
    data.fdwRevocationChecks = WTD_REVOKE_NONE;
    data.dwProvFlags = WTD_CACHE_ONLY_URL_RETRIEVAL;
    data.dwStateAction = WTD_STATEACTION_VERIFY;
    let raw = (data as *mut WINTRUST_DATA).cast::<c_void>();
    // SAFETY: `data` and what its union points to are alive for the call; no window is shown.
    let status = unsafe { WinVerifyTrust(HWND::default(), &mut action, raw) };
    let signer = (status == 0)
        .then(|| signer_name(data.hWVTStateData))
        .flatten();
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    // SAFETY: closes the state the verifying call left in `data`.
    let _ = unsafe { WinVerifyTrust(HWND::default(), &mut action, raw) };
    (status, signer)
}

/// The display name of the certificate that signed, from a verified state.
fn signer_name(state: HANDLE) -> Option<String> {
    // SAFETY: the state comes from a WinVerifyTrust call not yet closed; every pointer is checked
    // before use and the name is written within the buffer's length.
    unsafe {
        let provider = WTHelperProvDataFromStateData(state);
        if provider.is_null() {
            return None;
        }
        let signer = WTHelperGetProvSignerFromChain(provider, 0, false, 0);
        if signer.is_null() || (*signer).csCertChain == 0 || (*signer).pasCertChain.is_null() {
            return None;
        }
        let cert = (*(*signer).pasCertChain).pCert;
        if cert.is_null() {
            return None;
        }
        let mut name = [0u16; 256];
        let len = CertGetNameStringW(
            cert,
            CERT_NAME_SIMPLE_DISPLAY_TYPE,
            0,
            None,
            Some(&mut name),
        );
        (len > 1).then(|| String::from_utf16_lossy(&name[..len as usize - 1]))
    }
}

fn verdict(status: i32, signer: Option<String>) -> Signature {
    match (status, signer) {
        (0, signer) => Signature::Valid {
            signer: signer.unwrap_or_default(),
        },
        (TRUST_E_NOSIGNATURE | TRUST_E_SUBJECT_FORM_UNKNOWN | TRUST_E_PROVIDER_UNKNOWN, _) => {
            Signature::Unsigned
        }
        _ => Signature::Invalid,
    }
}

/// Checks the file's own signature, then the Windows catalogs, without reaching the network.
pub fn signature(path: &Path) -> Signature {
    let wide_path = wide(path);
    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(wide_path.as_ptr()),
        ..Default::default()
    };
    let mut data = WINTRUST_DATA {
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 {
            pFile: &mut file_info,
        },
        ..Default::default()
    };
    let (status, signer) = verify(&mut data);
    if status != TRUST_E_NOSIGNATURE {
        return verdict(status, signer);
    }
    catalog_signature(&wide_path).unwrap_or(Signature::Unsigned)
}

/// The signature a Windows catalog gives the file; `None` when no catalog lists it.
fn catalog_signature(wide_path: &[u16]) -> Option<Signature> {
    // SAFETY: opens the file for reading with a path that ends in a nul.
    let opened = unsafe {
        CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_SEQUENTIAL_SCAN,
            None,
        )
    };
    let file = Owned(opened.ok()?);
    let mut admin = 0isize;
    // SAFETY: fills `admin`, released below.
    let acquired = unsafe {
        CryptCATAdminAcquireContext2(&mut admin, None, BCRYPT_SHA256_ALGORITHM, None, None)
    };
    acquired.ok()?;
    let found = catalog_member(admin, file.0, wide_path);
    // SAFETY: releases the context acquired above, once.
    let _ = unsafe { CryptCATAdminReleaseContext(admin, 0) };
    found
}

/// Looks the file's hash up in the catalogs and verifies the catalog that lists it.
fn catalog_member(admin: isize, file: HANDLE, wide_path: &[u16]) -> Option<Signature> {
    let mut size = 0u32;
    // SAFETY: a call with no buffer only reports the hash size.
    let _ = unsafe { CryptCATAdminCalcHashFromFileHandle2(admin, file, &mut size, None, None) };
    let mut hash = vec![0u8; size as usize];
    // SAFETY: the buffer holds the `size` bytes asked for.
    let hashed = unsafe {
        CryptCATAdminCalcHashFromFileHandle2(admin, file, &mut size, Some(hash.as_mut_ptr()), None)
    };
    hashed.ok()?;
    // SAFETY: reads the hash slice; the catalog handle is released below.
    let catalog = unsafe { CryptCATAdminEnumCatalogFromHash(admin, &hash, None, None) };
    if catalog == 0 {
        return None;
    }
    let mut info = CATALOG_INFO {
        cbStruct: size_of::<CATALOG_INFO>() as u32,
        ..Default::default()
    };
    // SAFETY: writes the catalog's path into `info`, a struct of the size it declares.
    let read = unsafe { CryptCATCatalogInfoFromContext(catalog, &mut info, 0) };
    let result = read.ok().map(|()| {
        let tag: Vec<u16> = hash
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
            .encode_utf16()
            .chain([0])
            .collect();
        let mut member = WINTRUST_CATALOG_INFO {
            cbStruct: size_of::<WINTRUST_CATALOG_INFO>() as u32,
            pcwszCatalogFilePath: PCWSTR(info.wszCatalogFile.as_ptr()),
            pcwszMemberTag: PCWSTR(tag.as_ptr()),
            pcwszMemberFilePath: PCWSTR(wide_path.as_ptr()),
            hMemberFile: file,
            pbCalculatedFileHash: hash.as_mut_ptr(),
            cbCalculatedFileHash: size,
            hCatAdmin: admin,
            ..Default::default()
        };
        let mut data = WINTRUST_DATA {
            dwUnionChoice: WTD_CHOICE_CATALOG,
            Anonymous: WINTRUST_DATA_0 {
                pCatalog: &mut member,
            },
            ..Default::default()
        };
        let (status, signer) = verify(&mut data);
        verdict(status, signer)
    });
    // SAFETY: releases the catalog handle found above, once.
    let _ = unsafe { CryptCATAdminReleaseCatalogContext(admin, catalog, 0) };
    result
}
