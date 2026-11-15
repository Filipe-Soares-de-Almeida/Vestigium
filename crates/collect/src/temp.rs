use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use vestigium_core::health::TempSize;
use windows::Win32::Foundation::{ERROR_NO_MORE_FILES, HANDLE};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FIND_FIRST_EX_LARGE_FETCH, FindClose,
    FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW, WIN32_FIND_DATAW,
};
use windows::core::PCWSTR;

/// The most entries one count walks, which keeps a bloated folder from costing more than ~2 s.
pub const TEMP_MAX_ENTRIES: u32 = 300_000;
/// The longest one count may run.
pub const TEMP_MAX_TIME: Duration = Duration::from_secs(2);

/// The signed-in user's Temp folder (`%LOCALAPPDATA%\Temp`), when the variable is set.
pub fn temp_folder() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|local| PathBuf::from(local).join("Temp"))
}

/// Counts the folders and files under `root`, without following links and without opening any
/// file; it stops at `max_entries` or `max_time` and then reports the count as capped.
pub fn count_tree(root: &Path, max_entries: u32, max_time: Duration, ts_ms: i64) -> TempSize {
    let started = Instant::now();
    let mut size = TempSize {
        ts_ms,
        ..TempSize::default()
    };
    let mut pending = vec![long_path(root)];
    while let Some(dir) = pending.pop() {
        if size.folders.saturating_add(size.files) >= max_entries || started.elapsed() >= max_time {
            size.capped = true;
            break;
        }
        for_each_entry(&dir, |name, attributes| {
            let is_dir = attributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
            if is_dir {
                size.folders += 1;
                if attributes & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0 {
                    pending.push(dir.join(name));
                }
            } else {
                size.files += 1;
            }
        });
    }
    size
}

/// `path` with the `\\?\` prefix, so deep trees past 260 characters are walked too.
fn long_path(path: &Path) -> PathBuf {
    let text = path.as_os_str().to_string_lossy();
    if text.starts_with(r"\\?\") {
        path.to_path_buf()
    } else {
        PathBuf::from(format!(r"\\?\{text}"))
    }
}

/// Hands `each` the name and attributes of every entry of `dir` except `.` and `..`; a folder that
/// cannot be listed is skipped.
fn for_each_entry(dir: &Path, mut each: impl FnMut(std::ffi::OsString, u32)) {
    let pattern: Vec<u16> = dir
        .join("*")
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut data = WIN32_FIND_DATAW::default();
    // SAFETY: `pattern` is NUL-terminated and outlives the call; `data` receives the first entry.
    let found = unsafe {
        FindFirstFileExW(
            PCWSTR(pattern.as_ptr()),
            FindExInfoBasic,
            std::ptr::from_mut(&mut data).cast(),
            FindExSearchNameMatch,
            None,
            FIND_FIRST_EX_LARGE_FETCH,
        )
    };
    let Ok(handle) = found else { return };
    loop {
        let len = data
            .cFileName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(data.cFileName.len());
        let name = &data.cFileName[..len];
        if name != [u16::from(b'.')] && name != [u16::from(b'.'), u16::from(b'.')] {
            each(std::ffi::OsString::from_wide(name), data.dwFileAttributes);
        }
        // SAFETY: `handle` came from FindFirstFileExW and is still open.
        if let Err(err) = unsafe { FindNextFileW(handle, &mut data) } {
            debug_assert!(err.code() == ERROR_NO_MORE_FILES.to_hresult() || err.code().is_err());
            break;
        }
    }
    // SAFETY: closed once, here.
    let _ = unsafe { FindClose(HANDLE(handle.0)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vestigium-count-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["a/b/c", "a/d", "e"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in ["a/one.txt", "a/b/c/two.txt", "e/three.txt", "four.txt"] {
            std::fs::write(root.join(file), b"x").unwrap();
        }
        root
    }

    #[test]
    fn a_tree_is_counted_folder_by_folder() {
        let root = tree("all");
        let size = count_tree(&root, TEMP_MAX_ENTRIES, TEMP_MAX_TIME, 7);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            (size.folders, size.files, size.capped, size.ts_ms),
            (5, 4, false, 7)
        );
    }

    #[test]
    fn a_count_past_its_limit_is_capped() {
        let root = tree("capped");
        let size = count_tree(&root, 3, TEMP_MAX_TIME, 0);
        let _ = std::fs::remove_dir_all(&root);
        assert!(size.capped);
        assert!(size.folders + size.files >= 3);
    }

    #[test]
    fn a_missing_folder_counts_nothing() {
        let size = count_tree(
            Path::new(r"C:\no\such\folder\here"),
            TEMP_MAX_ENTRIES,
            TEMP_MAX_TIME,
            0,
        );
        assert_eq!((size.folders, size.files, size.capped), (0, 0, false));
    }
}
