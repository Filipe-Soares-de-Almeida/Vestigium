use std::path::PathBuf;

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_LocalAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

/// `%LOCALAPPDATA%\WatchMyPC\data`, kept apart from the installed binaries.
pub fn data_dir() -> windows::core::Result<PathBuf> {
    // SAFETY: returns a CoTaskMemAlloc'd path, freed below.
    let raw = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None)? };
    // SAFETY: `raw` is a NUL-terminated string from the call above.
    let local = String::from_utf16_lossy(unsafe { raw.as_wide() });
    // SAFETY: frees the string allocated by SHGetKnownFolderPath, exactly once.
    unsafe { CoTaskMemFree(Some(raw.0.cast_const().cast())) };
    Ok(PathBuf::from(local).join("WatchMyPC").join("data"))
}

/// The interface executable installed next to the agent, if there is one.
pub fn ui_exe() -> Option<PathBuf> {
    let path = std::env::current_exe()
        .ok()?
        .with_file_name("WatchMyPC.exe");
    path.is_file().then_some(path)
}
