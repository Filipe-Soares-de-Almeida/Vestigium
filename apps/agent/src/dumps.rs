use std::path::PathBuf;

use vestigium_collect::{
    CrashControl, DumpFile, app_dump_dir, crash_control, dump_file, list_dumps_one_level,
};
use vestigium_core::{DumpKind, DumpRecord, listing_changes};
use vestigium_store::{Store, StoreError};

/// Where the watcher looks for dumps it may see without admin.
#[derive(Clone, Debug, Default)]
pub struct DumpPlaces {
    /// Where app crash dumps go.
    pub app_dir: Option<PathBuf>,
    pub crash: Option<CrashControl>,
}

impl DumpPlaces {
    /// The places this PC is set up with.
    pub fn of_this_pc() -> DumpPlaces {
        DumpPlaces {
            app_dir: app_dump_dir(),
            crash: crash_control(),
        }
    }
}

/// Lists the dumps the watcher can see, stores the new ones and marks those gone; nothing is
/// opened. Returns how many rows changed.
pub fn take_inventory(
    store: &mut Store,
    places: &DumpPlaces,
    now_ms: i64,
) -> Result<usize, StoreError> {
    let mut listed = Vec::new();
    let mut covered = Vec::new();
    if let Some(dir) = &places.app_dir
        && let Some(listing) = list_dumps_one_level(dir)
    {
        if listing.complete {
            covered.push(with_separator(dir));
        }
        listed.extend(listing.files.into_iter().map(|f| record(f, DumpKind::App)));
    }
    if let Some(crash) = &places.crash {
        covered.push(crash.dump_file.to_string_lossy().into_owned());
        listed.extend(dump_file(&crash.dump_file).map(|f| record(f, DumpKind::KernelMemory)));
    }
    let present = store.present_dumps()?;
    let stored: Vec<(i64, &DumpRecord)> = present.iter().map(|r| (r.id, &r.record)).collect();
    let (fresh, gone) = listing_changes(&stored, &listed, &covered);
    if !fresh.is_empty() {
        store.upsert_dumps(&fresh, now_ms)?;
    }
    let marked = store.mark_dumps_deleted(&gone, now_ms)?;
    Ok(fresh.len() + marked)
}

/// A dump only listed: its file time stands in for the crash until the helper reads it.
fn record(file: DumpFile, kind: DumpKind) -> DumpRecord {
    DumpRecord {
        path: file.path.to_string_lossy().into_owned(),
        file_ms: file.file_ms,
        kind,
        crash_ms: file.file_ms,
        size_bytes: file.size_bytes,
        analysis: None,
    }
}

/// A folder path that only matches what is inside it.
fn with_separator(dir: &std::path::Path) -> String {
    let mut path = dir.to_string_lossy().into_owned();
    if !path.ends_with('\\') {
        path.push('\\');
    }
    path
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn places(dir: &std::path::Path) -> DumpPlaces {
        DumpPlaces {
            app_dir: Some(dir.join("CrashDumps")),
            crash: Some(CrashControl {
                dump_type: 7,
                dump_file: dir.join("MEMORY.DMP"),
                minidump_dir: dir.join("Minidump"),
                overwrite: true,
            }),
        }
    }

    #[test]
    fn inventory_stores_what_it_lists_and_marks_what_vanished() {
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().join("CrashDumps");
        fs::create_dir(&apps).unwrap();
        fs::write(apps.join("Game.exe.1.dmp"), [0u8; 64]).unwrap();
        fs::write(apps.join("Tool.exe.2.dmp"), [0u8; 32]).unwrap();
        fs::write(apps.join("notes.txt"), b"not a dump").unwrap();
        fs::write(dir.path().join("MEMORY.DMP"), [0u8; 128]).unwrap();
        let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let places = places(dir.path());

        assert_eq!(take_inventory(&mut store, &places, 1_000).unwrap(), 3);
        assert_eq!(
            take_inventory(&mut store, &places, 2_000).unwrap(),
            0,
            "nothing changed"
        );
        assert_eq!(store.dump_space().unwrap(), 64 + 32 + 128);

        fs::remove_file(apps.join("Tool.exe.2.dmp")).unwrap();
        assert_eq!(take_inventory(&mut store, &places, 3_000).unwrap(), 1);
        let present = store.present_dumps().unwrap();
        assert_eq!(present.len(), 2);
        assert!(
            present
                .iter()
                .any(|d| d.record.kind == DumpKind::KernelMemory)
        );
    }

    #[test]
    fn a_dump_in_a_crashdumps_subfolder_is_not_marked_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("CrashDumps").join("Game");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("a.dmp"), [0u8; 64]).unwrap();
        let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let stored = DumpRecord {
            path: sub.join("a.dmp").to_string_lossy().into_owned(),
            file_ms: 10,
            kind: DumpKind::App,
            crash_ms: 10,
            size_bytes: 64,
            analysis: None,
        };
        store.upsert_dumps(&[stored], 10).unwrap();
        let places = places(dir.path());

        take_inventory(&mut store, &places, 20).unwrap();
        take_inventory(&mut store, &places, 30).unwrap();

        let present = store.present_dumps().unwrap();
        assert_eq!(present.len(), 1);
        assert_eq!(present[0].record.path, sub.join("a.dmp").to_string_lossy());

        fs::remove_file(sub.join("a.dmp")).unwrap();
        take_inventory(&mut store, &places, 40).unwrap();
        assert!(
            store.present_dumps().unwrap().is_empty(),
            "gone is still gone"
        );
    }

    #[test]
    fn a_folder_it_cannot_list_loses_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let unread = DumpRecord {
            path: dir
                .path()
                .join("Minidump")
                .join("1.dmp")
                .to_string_lossy()
                .into_owned(),
            file_ms: 10,
            kind: DumpKind::KernelMini,
            crash_ms: 10,
            size_bytes: 1,
            analysis: None,
        };
        store.upsert_dumps(&[unread], 10).unwrap();

        take_inventory(&mut store, &places(dir.path()), 20).unwrap();

        assert_eq!(store.present_dumps().unwrap().len(), 1);
    }
}
