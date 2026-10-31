use std::path::PathBuf;

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_LocalAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

/// `%LOCALAPPDATA%\Vestigium\data`, kept apart from the installed binaries.
pub fn data_dir() -> windows::core::Result<PathBuf> {
    // SAFETY: returns a CoTaskMemAlloc'd path, freed below.
    let raw = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None)? };
    // SAFETY: `raw` is a NUL-terminated string from the call above.
    let local = String::from_utf16_lossy(unsafe { raw.as_wide() });
    // SAFETY: frees the string allocated by SHGetKnownFolderPath, exactly once.
    unsafe { CoTaskMemFree(Some(raw.0.cast_const().cast())) };
    Ok(PathBuf::from(local).join("Vestigium").join("data"))
}

/// The window executable next to the agent: the installed name, then the development build name.
pub fn ui_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    ["Vestigium.Window.exe", "vestigium-window.exe"]
        .into_iter()
        .map(|name| exe.with_file_name(name))
        .find(|path| path.is_file())
}
