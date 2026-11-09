use std::io::Write;
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf, Prefix};
use std::process::ExitCode;

use vestigium_collect::{app_dump_dir, crash_control, wall_ms};
use vestigium_core::{DUMP_DELETION_VERSION, DumpDeletion};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

use crate::{job, output};

/// Most paths one list may name.
const MOST_PATHS: usize = 500;

/// Where a dump may be deleted from.
#[derive(Clone, Debug, Default)]
pub struct DeletePlaces {
    /// Folders whose `.dmp` files may go, but not those of their subfolders.
    pub flat: Vec<PathBuf>,
    /// Folders whose `.dmp` files may go, and those one folder down.
    pub deep: Vec<PathBuf>,
    /// Single files that may go, such as `MEMORY.DMP`.
    pub files: Vec<PathBuf>,
}

impl DeletePlaces {
    fn of_this_pc() -> DeletePlaces {
        let mut places = DeletePlaces::default();
        if let Some(crash) = crash_control() {
            places.flat.push(crash.minidump_dir);
            places.files.push(crash.dump_file);
        }
        if let Some(root) = std::env::var_os("SystemRoot") {
            places
                .deep
                .push(PathBuf::from(root).join("LiveKernelReports"));
        }
        places.deep.extend(app_dump_dir());
        places
    }
}

/// Deletes the dumps a `.delete` list names, each checked again here, and writes what happened
/// to a `.deleted` file beside the list.
pub fn run(list: &Path) -> ExitCode {
    if output::check_existing(list, "dumps", "delete").is_err() {
        return ExitCode::from(job::REFUSED);
    }
    let Ok(text) = std::fs::read_to_string(list) else {
        return ExitCode::from(job::REFUSED);
    };
    let Ok(mut file) = output::create(&list.with_extension("deleted"), "dumps", "deleted") else {
        return ExitCode::from(job::REFUSED);
    };
    let places = DeletePlaces::of_this_pc();
    let mut done = DumpDeletion {
        ts_ms: wall_ms(),
        deleted: Vec::new(),
        refused: Vec::new(),
        failed: Vec::new(),
    };
    for line in paths_in(&text) {
        let path = Path::new(line);
        if allowed(path, &places).is_err() {
            done.refused.push(line.to_string());
        } else if std::fs::remove_file(path).is_ok() {
            done.deleted.push(line.to_string());
        } else {
            done.failed.push(line.to_string());
        }
    }
    let mut bytes = vec![DUMP_DELETION_VERSION];
    match postcard::to_allocvec(&done) {
        Ok(body) => bytes.extend(body),
        Err(_) => return ExitCode::from(job::WRITE_FAILED),
    }
    if file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        return ExitCode::from(job::WRITE_FAILED);
    }
    ExitCode::SUCCESS
}

/// The paths a list names, one per line, blank lines left out, at most `MOST_PATHS`.
pub fn paths_in(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(MOST_PATHS)
}

/// Whether `path` is a `.dmp` file in one of `places`, local, absolute, with no `.`, `..`, link
/// or junction on the way.
pub fn allowed(path: &Path, places: &DeletePlaces) -> Result<(), String> {
    let mut parts = path.components();
    match parts.next() {
        Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)) => {}
        _ => return Err(format!("{} is not on a local drive", path.display())),
    }
    if parts.next() != Some(Component::RootDir) {
        return Err(format!("{} is not absolute", path.display()));
    }
    if parts.any(|p| !matches!(p, Component::Normal(_))) {
        return Err(format!("{} has a relative part", path.display()));
    }
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("dmp"))
    {
        return Err(format!("{} is not a .dmp file", path.display()));
    }
    let parent = path.parent().ok_or("no folder")?;
    let inside = places.files.iter().any(|f| same(f, path))
        || places.flat.iter().any(|d| same(d, parent))
        || places
            .deep
            .iter()
            .any(|d| same(d, parent) || parent.parent().is_some_and(|up| same(d, up)));
    if !inside {
        return Err(format!("{} is outside the dump folders", path.display()));
    }
    for step in path.ancestors() {
        if step.parent().is_none() {
            break;
        }
        let meta = std::fs::symlink_metadata(step).map_err(|e| e.to_string())?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(format!("{} is a link", step.display()));
        }
    }
    if !std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file()) {
        return Err(format!("{} is not a file", path.display()));
    }
    Ok(())
}

fn same(a: &Path, b: &Path) -> bool {
    a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use super::*;

    struct Setup {
        dir: tempfile::TempDir,
        places: DeletePlaces,
    }

    fn setup() -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for folder in [
            "Minidump",
            "LiveKernelReports\\WATCHDOG",
            "CrashDumps",
            "Other",
        ] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        for file in [
            "Minidump\\1.dmp",
            "LiveKernelReports\\WATCHDOG\\WD-1.dmp",
            "CrashDumps\\Game.exe.1.dmp",
            "CrashDumps\\notes.txt",
            "Other\\2.dmp",
            "MEMORY.DMP",
        ] {
            fs::write(root.join(file), b"x").unwrap();
        }
        let places = DeletePlaces {
            flat: vec![root.join("Minidump")],
            deep: vec![root.join("LiveKernelReports"), root.join("CrashDumps")],
            files: vec![root.join("MEMORY.DMP")],
        };
        Setup { dir, places }
    }

    #[test]
    fn accepts_dumps_in_the_known_folders() {
        let s = setup();
        let root = s.dir.path();
        for ok in [
            "Minidump\\1.dmp",
            "LiveKernelReports\\WATCHDOG\\WD-1.dmp",
            "CrashDumps\\Game.exe.1.dmp",
            "MEMORY.DMP",
        ] {
            assert_eq!(allowed(&root.join(ok), &s.places), Ok(()), "{ok}");
        }
    }

    #[test]
    fn refuses_a_path_outside_the_dump_folders() {
        let s = setup();
        let root = s.dir.path();
        for bad in [
            root.join("Other\\2.dmp"),
            root.join("Minidump\\..\\Other\\2.dmp"),
            PathBuf::from("Minidump\\1.dmp"),
            PathBuf::from(r"\\server\share\Minidump\1.dmp"),
            root.join("Minidump\\missing.dmp"),
        ] {
            assert!(allowed(&bad, &s.places).is_err(), "{}", bad.display());
        }
    }

    #[test]
    fn refuses_anything_but_dmp() {
        let s = setup();
        let notes = s.dir.path().join("CrashDumps\\notes.txt");
        assert!(allowed(&notes, &s.places).is_err());
        let folder = s.dir.path().join("Minidump");
        assert!(allowed(&folder, &s.places).is_err());
    }

    #[test]
    fn refuses_a_link_on_the_way() {
        let s = setup();
        let root = s.dir.path();
        let link = root.join("CrashDumps\\Linked");
        let made = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(root.join("Other"))
            .output()
            .unwrap();
        assert!(made.status.success(), "could not make a junction");
        assert!(allowed(&link.join("2.dmp"), &s.places).is_err());
    }

    #[test]
    fn a_list_names_one_path_per_line() {
        let text = "C:\\a.dmp\r\n\r\n  C:\\b.dmp  \n";
        assert_eq!(
            paths_in(text).collect::<Vec<_>>(),
            ["C:\\a.dmp", "C:\\b.dmp"]
        );
        let long = "C:\\a.dmp\n".repeat(MOST_PATHS + 10);
        assert_eq!(paths_in(&long).count(), MOST_PATHS);
    }
}
