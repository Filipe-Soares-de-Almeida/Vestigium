use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Component, Path, PathBuf, Prefix};

use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    DELETE, FILE_ATTRIBUTE_REPARSE_POINT, FILE_DISPOSITION_INFO, FILE_GENERIC_WRITE,
    FILE_NAME_NORMALIZED, FileDispositionInfo, GETFINALPATHNAMEBYHANDLE_FLAGS,
    GetFinalPathNameByHandleW, GetLongPathNameW, SetFileInformationByHandle, VOLUME_NAME_DOS,
};
use windows::core::HSTRING;

/// Checks that `path` is `<drive>:\...\Vestigium\data\<folder>\<digits>.<extension>`, with no `.` or `..`.
pub fn check_shape(path: &Path, folder: &str, extension: &str) -> Result<(), String> {
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
        return Err(format!("{file} is not a numbered result"));
    }
    if !names.ends_with(&["vestigium".into(), "data".into(), folder.to_lowercase()]) {
        return Err(format!(
            "{} is outside a Vestigium {folder} folder",
            path.display()
        ));
    }
    Ok(())
}

/// Checks that `path` names an existing file of the right shape, with no junction or link on the way.
pub fn check_existing(path: &Path, folder: &str, extension: &str) -> Result<(), String> {
    check_shape(path, folder, extension)?;
    for step in path.ancestors() {
        if step.parent().is_none() {
            break;
        }
        let meta = std::fs::symlink_metadata(step).map_err(|e| e.to_string())?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(format!("{} is a link", step.display()));
        }
    }
    if std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file()) {
        Ok(())
    } else {
        Err(format!("{} is not a file", path.display()))
    }
}

/// Creates the result file: right shape, every folder on the way real (no junction or link), and new.
pub fn create(path: &Path, folder: &str, extension: &str) -> Result<File, String> {
    check_shape(path, folder, extension)?;
    for folder in path.ancestors().skip(1) {
        if folder.parent().is_none() {
            break;
        }
        let meta = std::fs::symlink_metadata(folder).map_err(|e| e.to_string())?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(format!("{} is a link", folder.display()));
        }
    }
    let file = OpenOptions::new()
        .write(true)
        .access_mode(FILE_GENERIC_WRITE.0 | DELETE.0)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    if let Err(reason) = same_final_path(&file, path) {
        let _ = delete_through(&file);
        return Err(reason);
    }
    Ok(file)
}

/// Checks that the open `file` really lives at `asked`, comparing the final path its handle reports.
pub fn same_final_path(file: &File, asked: &Path) -> Result<(), String> {
    let mut buffer = vec![0u16; 1024];
    // SAFETY: the handle belongs to the open `file`; the buffer length is passed with it.
    let len = unsafe {
        GetFinalPathNameByHandleW(
            HANDLE(file.as_raw_handle()),
            &mut buffer,
            GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
        )
    } as usize;
    if len == 0 || len >= buffer.len() {
        return Err(format!("cannot read the final path of {}", asked.display()));
    }
    let actual = normal(&PathBuf::from(String::from_utf16_lossy(&buffer[..len])));
    if actual == normal(&long_form(asked)) {
        Ok(())
    } else {
        Err(format!("{} leads somewhere else", asked.display()))
    }
}

/// The path with every 8.3 short name spelled out; the path itself when it cannot be expanded.
fn long_form(path: &Path) -> PathBuf {
    let wide = HSTRING::from(path.as_os_str());
    let mut buffer = vec![0u16; 1024];
    // SAFETY: a NUL-terminated input string and a buffer whose length is passed with it.
    let len = unsafe { GetLongPathNameW(&wide, Some(&mut buffer)) } as usize;
    if len == 0 || len >= buffer.len() {
        return path.to_path_buf();
    }
    PathBuf::from(String::from_utf16_lossy(&buffer[..len]))
}

/// The path in lower case without the extended-length prefix, to compare two spellings of one place.
fn normal(path: &Path) -> String {
    let text = path.to_string_lossy().to_lowercase();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

/// Deletes the file through its own handle, which must have been opened with `DELETE` access;
/// it goes when the last handle closes.
pub fn delete_through(file: &File) -> Result<(), String> {
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: a live disposition struct of the size passed, on the handle of the open `file`.
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            (&raw const info).cast::<c_void>(),
            size_of_val(&info) as u32,
        )
    }
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str =
        r"C:\Users\someone\AppData\Local\Vestigium\data\captures\1793580000000.capture";

    #[test]
    fn accepts_a_numbered_capture_in_the_captures_folder() {
        assert_eq!(check_shape(Path::new(GOOD), "captures", "capture"), Ok(()));
        let upper = GOOD.to_uppercase();
        assert_eq!(
            check_shape(Path::new(&upper), "captures", "capture"),
            Ok(())
        );
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
            assert!(
                check_shape(Path::new(bad), "captures", "capture").is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn refuses_scan_output_outside_scans() {
        let good = r"C:\Users\someone\AppData\Local\Vestigium\data\scans\1793580000000.scan";
        assert_eq!(check_shape(Path::new(good), "scans", "scan"), Ok(()));
        for bad in [
            r"C:\Users\someone\AppData\Local\Vestigium\data\captures\1.scan",
            r"C:\Users\someone\AppData\Local\Vestigium\data\scans\1.capture",
            r"C:\Users\someone\AppData\Local\Vestigium\data\scans\..\1.scan",
            r"C:\Windows\Temp\scans\1.scan",
        ] {
            assert!(
                check_shape(Path::new(bad), "scans", "scan").is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn creates_only_new_files() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("Vestigium").join("data").join("captures");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("42.capture");
        assert!(create(&path, "captures", "capture").is_ok());
        assert!(create(&path, "captures", "capture").is_err());
    }

    #[test]
    fn verify_final_path_refuses_a_redirected_file() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let link = dir.path().join("link");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&real)
            .output()
            .unwrap();
        assert!(made.status.success(), "could not make a junction");
        let asked = link.join("1.capture");
        let file = OpenOptions::new()
            .write(true)
            .access_mode(FILE_GENERIC_WRITE.0 | DELETE.0)
            .create_new(true)
            .open(&asked)
            .unwrap();
        assert!(same_final_path(&file, &asked).is_err());
        let plain = real.join("2.capture");
        let file = File::create_new(&plain).unwrap();
        assert_eq!(same_final_path(&file, &plain), Ok(()));
    }

    #[test]
    fn deletes_a_file_through_its_handle() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("Vestigium").join("data").join("captures");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("7.capture");
        let file = create(&path, "captures", "capture").unwrap();
        assert!(delete_through(&file).is_ok());
        drop(file);
        assert!(!path.exists());
    }
}
