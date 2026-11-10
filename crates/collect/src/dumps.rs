use std::fs::Metadata;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
use windows::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, REG_ROUTINE_FLAGS, RRF_RT_REG_DWORD, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
    RegGetValueW,
};
use windows::core::{PCWSTR, w};

const CRASH_CONTROL: PCWSTR = w!(r"SYSTEM\CurrentControlSet\Control\CrashControl");
const LOCAL_DUMPS: PCWSTR = w!(r"SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps");

/// What Windows is set to write when it stops with a blue screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrashControl {
    /// `CrashDumpEnabled`: 0 none, 1 complete, 2 kernel, 3 small, 7 automatic.
    pub dump_type: u32,
    /// Where the kernel or complete dump goes, usually `MEMORY.DMP` in the Windows folder.
    pub dump_file: PathBuf,
    pub minidump_dir: PathBuf,
    /// A new memory dump replaces the old one.
    pub overwrite: bool,
}

/// One dump file as its folder lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpFile {
    pub path: PathBuf,
    /// Last write, Unix epoch ms.
    pub file_ms: i64,
    pub size_bytes: u64,
}

/// The crash dump settings, readable without admin; `None` when the key cannot be read.
pub fn crash_control() -> Option<CrashControl> {
    Some(CrashControl {
        dump_type: dword(CRASH_CONTROL, w!("CrashDumpEnabled"))?,
        dump_file: PathBuf::from(text(CRASH_CONTROL, w!("DumpFile"))?),
        minidump_dir: PathBuf::from(text(CRASH_CONTROL, w!("MinidumpDir"))?),
        overwrite: dword(CRASH_CONTROL, w!("Overwrite")).unwrap_or(1) != 0,
    })
}

/// Where Windows Error Reporting keeps app crash dumps: its `DumpFolder`, else `CrashDumps`
/// under the user's local app data.
pub fn app_dump_dir() -> Option<PathBuf> {
    text(LOCAL_DUMPS, w!("DumpFolder"))
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .filter(|v| !v.is_empty())
                .map(|local| PathBuf::from(local).join("CrashDumps"))
        })
}

/// The `.dmp` files in `dir`; `None` when the folder cannot be listed, so nothing in it can be
/// called gone.
pub fn list_dumps(dir: &Path) -> Option<Vec<DumpFile>> {
    let entries = std::fs::read_dir(dir).ok()?;
    Some(
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("dmp"))
            })
            .filter_map(|path| dump_file(&path))
            .collect(),
    )
}

/// The dumps of a folder and of the folders right under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpListing {
    pub files: Vec<DumpFile>,
    /// Every folder was listed, so a dump absent from `files` is really gone.
    pub complete: bool,
}

/// The `.dmp` files in `dir` and in its real subfolders; `None` when `dir` cannot be listed.
pub fn list_dumps_one_level(dir: &Path) -> Option<DumpListing> {
    let mut files = list_dumps(dir)?;
    let mut complete = true;
    for sub in subfolders(dir) {
        match list_dumps(&sub) {
            Some(found) => files.extend(found),
            None => complete = false,
        }
    }
    Some(DumpListing { files, complete })
}

/// The real folders directly under `dir`: a junction or link is never one.
pub fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            std::fs::symlink_metadata(entry.path()).is_ok_and(|m| m.is_dir() && !is_link(&m))
        })
        .map(|entry| entry.path())
        .collect()
}

/// Whether the entry is a junction, symbolic link or other reparse point.
fn is_link(meta: &Metadata) -> bool {
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
}

/// Size and last write of one dump, from its metadata; the file is never opened, and a link is not one.
pub fn dump_file(path: &Path) -> Option<DumpFile> {
    let meta = std::fs::symlink_metadata(path)
        .ok()
        .filter(|m| m.is_file() && !is_link(m))?;
    let file_ms = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    Some(DumpFile {
        path: path.to_path_buf(),
        file_ms: i64::try_from(file_ms).ok()?,
        size_bytes: meta.len(),
    })
}

fn dword(key: PCWSTR, value: PCWSTR) -> Option<u32> {
    let (mut data, mut size) = (0u32, 4u32);
    // SAFETY: reads one DWORD into a buffer of the size passed; both strings are static.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast()),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

/// A string value, with `%SystemRoot%` and the like expanded.
fn text(key: PCWSTR, value: PCWSTR) -> Option<String> {
    let flags = REG_ROUTINE_FLAGS(RRF_RT_REG_SZ.0 | RRF_RT_REG_EXPAND_SZ.0);
    let mut size = 0u32;
    // SAFETY: a size query with no buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key,
            value,
            flags,
            None,
            None,
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS || size == 0 {
        return None;
    }
    let mut data = vec![0u16; size as usize / 2 + 1];
    let mut bytes = (data.len() * 2) as u32;
    // SAFETY: `data` has `bytes` writable bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key,
            value,
            flags,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let end = data.iter().position(|&u| u == 0).unwrap_or(data.len());
    Some(String::from_utf16_lossy(&data[..end])).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_folder_lists_its_dumps_and_subfolders() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Sub")).unwrap();
        std::fs::write(dir.path().join("a.dmp"), b"x").unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        let listed = list_dumps(dir.path()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].size_bytes, 1);
        assert_eq!(subfolders(dir.path()), [dir.path().join("Sub")]);
    }

    #[test]
    fn the_one_level_listing_reads_the_folder_and_its_subfolders() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("Game");
        std::fs::create_dir_all(sub.join("Deeper")).unwrap();
        std::fs::write(dir.path().join("a.dmp"), b"x").unwrap();
        std::fs::write(sub.join("b.dmp"), b"xy").unwrap();
        std::fs::write(sub.join("Deeper").join("c.dmp"), b"xyz").unwrap();
        std::fs::write(sub.join("notes.txt"), b"x").unwrap();

        let listing = list_dumps_one_level(dir.path()).unwrap();

        let mut names: Vec<_> = listing
            .files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["a.dmp", "b.dmp"], "one level down, no deeper");
        assert!(listing.complete);
        assert!(list_dumps_one_level(&dir.path().join("missing")).is_none());
    }

    #[test]
    fn a_junction_is_not_a_subfolder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Real")).unwrap();
        std::fs::create_dir(dir.path().join("Target")).unwrap();
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(dir.path().join("Linked"))
            .arg(dir.path().join("Target"))
            .output()
            .unwrap();
        assert!(made.status.success(), "could not make a junction");
        let mut found = subfolders(dir.path());
        found.sort();
        assert_eq!(found, [dir.path().join("Real"), dir.path().join("Target")]);
    }
}
