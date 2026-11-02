use std::fs::{File, OpenOptions};
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, Prefix};

use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

/// Checks that `path` is `<drive>:\...\Vestigium\data\captures\<digits>.<extension>`, with no `.` or `..`.
pub fn check_shape(path: &Path, extension: &str) -> Result<(), String> {
    let mut parts = path.components();
    match parts.next() {
        Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)) => {}
        _ => return Err(format!("{} is not on a local drive", path.display())),
    }
    if parts.next() != Some(Component::RootDir) {
        return Err(format!("{} is not absolute", path.display()));
    }
    let mut names = Vec::new();
    for part in parts {
        match part {
            Component::Normal(name) => names.push(name.to_string_lossy().to_lowercase()),
            _ => return Err(format!("{} has a relative part", path.display())),
        }
    }
    let file = names.pop().ok_or("no file name")?;
    let stem = file
        .strip_suffix(&format!(".{extension}"))
        .ok_or_else(|| format!("{file} is not a .{extension} file"))?;
    if stem.is_empty() || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{file} is not a numbered capture"));
    }
    if !names.ends_with(&["vestigium".into(), "data".into(), "captures".into()]) {
        return Err(format!(
            "{} is outside a Vestigium captures folder",
            path.display()
        ));
    }
    Ok(())
}

/// Creates the capture file: right shape, every folder on the way real (no junction or link), and new.
pub fn create(path: &Path, extension: &str) -> Result<File, String> {
    check_shape(path, extension)?;
    for folder in path.ancestors().skip(1) {
        if folder.parent().is_none() {
            break;
        }
        let meta = std::fs::symlink_metadata(folder).map_err(|e| e.to_string())?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(format!("{} is a link", folder.display()));
        }
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str =
        r"C:\Users\someone\AppData\Local\Vestigium\data\captures\1793580000000.capture";

    #[test]
    fn accepts_a_numbered_capture_in_the_captures_folder() {
        assert_eq!(check_shape(Path::new(GOOD), "capture"), Ok(()));
        let upper = GOOD.to_uppercase();
        assert_eq!(check_shape(Path::new(&upper), "capture"), Ok(()));
    }

    #[test]
    fn refuses_output_outside_captures() {
        for bad in [
            r"C:\Windows\System32\1.capture",
            r"C:\Users\someone\AppData\Local\Vestigium\data\1.capture",
            r"C:\Users\someone\AppData\Local\Vestigium\data\captures\..\..\1.capture",
            r"C:\Users\someone\AppData\Local\Vestigium\data\captures\1.exe",
            r"C:\Users\someone\AppData\Local\Vestigium\data\captures\name.capture",
            r"C:\Users\someone\AppData\Local\Vestigium\data\captures\.capture",
            r"Vestigium\data\captures\1.capture",
            r"\\server\share\Vestigium\data\captures\1.capture",
            r"\\?\C:\Users\someone\AppData\Local\Vestigium\data\captures\1.capture",
        ] {
            assert!(check_shape(Path::new(bad), "capture").is_err(), "{bad}");
        }
    }

    #[test]
    fn creates_only_new_files() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("Vestigium").join("data").join("captures");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("42.capture");
        assert!(create(&path, "capture").is_ok());
        assert!(create(&path, "capture").is_err());
    }
}
