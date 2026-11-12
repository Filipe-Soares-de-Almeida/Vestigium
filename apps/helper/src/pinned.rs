use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use vestigium_collect::sha256_hex;

use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;

/// The PresentMon console the helper runs, beside its own executable.
pub const PRESENTMON_FILE: &str = "PresentMon-2.6.0-x64.exe";
const PRESENTMON_SHA256: &str = "b2a706bc6ad475749e3b7e3409263aa1e6906d45bdcf993f6dbc0f660188f1af";

/// The checked PresentMon and an open handle that keeps anyone from changing it while it runs.
#[derive(Debug)]
pub struct Pinned {
    pub path: PathBuf,
    _lock: File,
}

/// PresentMon in `dir`, if its SHA-256 is the pinned one.
pub fn presentmon(dir: &Path) -> Result<Pinned, String> {
    check(&dir.join(PRESENTMON_FILE), PRESENTMON_SHA256)
}

fn check(path: &Path, expected: &str) -> Result<Pinned, String> {
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let found = sha256_hex(&bytes)?;
    if found != expected {
        return Err(format!(
            "{} has SHA-256 {found}, not the pinned {expected}",
            path.display()
        ));
    }
    Ok(Pinned {
        path: path.to_path_buf(),
        _lock: file,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_presentmon_with_wrong_hash() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(PRESENTMON_FILE), b"not the real one").unwrap();
        let err = presentmon(dir.path()).unwrap_err();
        assert!(err.contains("not the pinned"), "{err}");
    }

    #[test]
    fn missing_presentmon_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(presentmon(dir.path()).is_err());
    }

    #[test]
    fn checked_file_cannot_be_rewritten_while_held() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tool.exe");
        std::fs::write(&path, b"abc").unwrap();
        let held = check(
            &path,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        )
        .unwrap();
        assert!(std::fs::write(&path, b"changed").is_err());
        drop(held);
        assert!(std::fs::write(&path, b"changed").is_ok());
    }
}
