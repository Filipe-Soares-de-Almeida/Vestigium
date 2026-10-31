use std::path::PathBuf;

/// `%LOCALAPPDATA%\Vestigium\data`, kept apart from the installed binaries.
pub fn data_dir() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty())?;
    Some(PathBuf::from(local).join("Vestigium").join("data"))
}

/// The window executable next to the agent: the installed name, then the development build name.
pub fn ui_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    ["Vestigium.Window.exe", "vestigium-window.exe"]
        .into_iter()
        .map(|name| exe.with_file_name(name))
        .find(|path| path.is_file())
}
