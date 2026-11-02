use std::fs;
use std::path::{Path, PathBuf};

use vestigium_core::health::{SCAN_VERSION, ScanResult};
use vestigium_core::{CAPTURE_VERSION, DeepCapture, ProbeHiccup};
use vestigium_store::Store;

/// Stores the finished capture files of `dir` and deletes them; returns the ids in name order.
/// A file that does not decode is kept, since the helper writes it only when its capture ends.
pub fn import_captures(dir: &Path, hiccups: &[ProbeHiccup], store: &mut Store) -> Vec<i64> {
    let mut ids = Vec::new();
    for path in numbered_files(dir, "capture") {
        let Some(mut capture) = read_capture(&path) else {
            continue;
        };
        capture.hiccups.extend(
            hiccups
                .iter()
                .filter(|h| (capture.start_ms..=capture.end_ms).contains(&h.ts_ms))
                .copied(),
        );
        let Ok(id) = store.insert_capture(&capture) else {
            continue;
        };
        ids.push(id);
        let _ = fs::remove_file(&path);
    }
    ids
}

/// Stores the health scan files of `dir` and deletes them; returns the ids in name order.
pub fn import_scans(dir: &Path, store: &mut Store) -> Vec<i64> {
    let mut ids = Vec::new();
    for path in numbered_files(dir, "scan") {
        let Some(scan) = read_versioned::<ScanResult>(&path, SCAN_VERSION) else {
            continue;
        };
        let Ok(id) = store.insert_scan(&scan) else {
            continue;
        };
        ids.push(id);
        let _ = fs::remove_file(&path);
    }
    ids
}

/// The files named `<digits>.<extension>`, in name order.
fn numbered_files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .filter(|path| is_numbered(path, extension))
        .collect();
    files.sort();
    files
}

fn is_numbered(path: &Path, extension: &str) -> bool {
    let digits = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(&format!(".{extension}")));
    digits.is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

fn read_capture(path: &Path) -> Option<DeepCapture> {
    read_versioned(path, CAPTURE_VERSION)
}

/// A file written as one version byte and a postcard body; `None` for another version.
fn read_versioned<T: serde::de::DeserializeOwned>(path: &Path, expected: u8) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    let (&version, body) = bytes.split_first()?;
    if version != expected {
        return None;
    }
    postcard::from_bytes(body).ok()
}

#[cfg(test)]
mod tests {
    use vestigium_core::CaptureSource;

    use super::*;

    fn capture(start_ms: i64, end_ms: i64) -> DeepCapture {
        DeepCapture {
            start_ms,
            end_ms,
            kernel: CaptureSource::Ok,
            frames: CaptureSource::Ok,
            lost_events: 0,
            drivers: Vec::new(),
            longest: Vec::new(),
            apps: Vec::new(),
            hitches: Vec::new(),
            hiccups: Vec::new(),
        }
    }

    fn write_capture(dir: &Path, name: &str, capture: &DeepCapture) {
        let mut bytes = vec![CAPTURE_VERSION];
        bytes.extend(postcard::to_allocvec(capture).unwrap());
        fs::write(dir.join(name), bytes).unwrap();
    }

    fn store(dir: &Path) -> Store {
        Store::open(&dir.join("vestigium.db")).unwrap()
    }

    fn hiccup(ts_ms: i64) -> ProbeHiccup {
        ProbeHiccup {
            ts_ms,
            lag_ms: 12.0,
        }
    }

    #[test]
    fn import_reads_only_numbered_capture_files() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        write_capture(&captures, "20.capture", &capture(2_000, 3_000));
        write_capture(&captures, "10.capture", &capture(1_000, 1_500));
        write_capture(&captures, "notes.capture", &capture(5_000, 6_000));
        write_capture(&captures, "30.capture.tmp", &capture(7_000, 8_000));
        write_capture(&captures, "40.etl", &capture(9_000, 9_500));
        write_capture(&captures, ".capture", &capture(9_000, 9_500));
        fs::create_dir(captures.join("50.capture")).unwrap();
        let mut store = store(dir.path());

        let ids = import_captures(&captures, &[], &mut store);

        let stored: Vec<i64> = store
            .captures(i64::MIN, i64::MAX)
            .unwrap()
            .iter()
            .map(|c| c.start_ms)
            .collect();
        assert_eq!(ids.len(), 2);
        assert_eq!(stored, [1_000, 2_000], "read in name order");
        assert!(!captures.join("10.capture").exists());
        assert!(!captures.join("20.capture").exists());
        for kept in [
            "notes.capture",
            "30.capture.tmp",
            "40.etl",
            ".capture",
            "50.capture",
        ] {
            assert!(captures.join(kept).exists(), "{kept} is not ours to take");
        }
    }

    #[test]
    fn import_keeps_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        fs::write(captures.join("1.capture"), b"").unwrap();
        fs::write(captures.join("2.capture"), [CAPTURE_VERSION + 1, 0, 1, 2]).unwrap();
        fs::write(captures.join("3.capture"), [CAPTURE_VERSION, 0xff, 0xff]).unwrap();
        let mut store = store(dir.path());

        let ids = import_captures(&captures, &[], &mut store);

        assert!(ids.is_empty());
        assert!(store.captures(i64::MIN, i64::MAX).unwrap().is_empty());
        for name in ["1.capture", "2.capture", "3.capture"] {
            assert!(captures.join(name).exists(), "{name} may still be written");
        }
    }

    #[test]
    fn import_adds_the_hiccups_of_its_window() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        write_capture(&captures, "1.capture", &capture(1_000, 2_000));
        let mut store = store(dir.path());
        let seen = [
            hiccup(999),
            hiccup(1_000),
            hiccup(1_500),
            hiccup(2_000),
            hiccup(2_001),
        ];

        let ids = import_captures(&captures, &seen, &mut store);

        let stored = store.capture(ids[0]).unwrap().unwrap();
        let times: Vec<i64> = stored.hiccups.iter().map(|h| h.ts_ms).collect();
        assert_eq!(times, [1_000, 1_500, 2_000]);
    }

    #[test]
    fn import_twice_is_one_row() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        write_capture(&captures, "1.capture", &capture(1_000, 2_000));
        let mut store = store(dir.path());

        let first = import_captures(&captures, &[], &mut store);
        let second = import_captures(&captures, &[], &mut store);

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(store.captures(i64::MIN, i64::MAX).unwrap().len(), 1);
    }

    #[test]
    fn import_without_a_folder_stores_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(dir.path());

        assert!(import_captures(&dir.path().join("missing"), &[], &mut store).is_empty());
    }
}
