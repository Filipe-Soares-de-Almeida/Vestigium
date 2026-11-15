use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use crate::chkdsk::system_directory;
use crate::{job, output};

/// Largest trace the helper copies; a bigger file is refused.
pub const SOURCE_MAX_BYTES: u64 = 512 * 1024 * 1024;
/// How much of the trace is read and written at a time.
const CHUNK: usize = 1024 * 1024;

/// Why a copy of the trace did not happen.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// The output is not acceptable, or the source is a link, not a file or too big.
    Refused,
    /// Windows has no trace file.
    Missing,
    /// The trace could not be read or the copy could not be written.
    WriteFailed,
}

impl Failure {
    fn exit_code(&self) -> ExitCode {
        ExitCode::from(match self {
            Failure::Refused => job::REFUSED,
            Failure::Missing => job::TRACE_MISSING,
            Failure::WriteFailed => job::WRITE_FAILED,
        })
    }
}

/// `<system dir>\WDI\LogFiles\BootPerfDiagLogger.etl`, the one file this job reads.
pub fn source_path(system: &Path) -> PathBuf {
    system
        .join("WDI")
        .join("LogFiles")
        .join("BootPerfDiagLogger.etl")
}

/// Copies the boot trace into `out`, a new `<digits>.etl` in the boot traces folder.
pub fn run(out: &Path) -> ExitCode {
    let Some(system) = system_directory() else {
        return ExitCode::from(job::REFUSED);
    };
    match copy_trace(&source_path(&system), out, SOURCE_MAX_BYTES) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.exit_code(),
    }
}

/// Copies `source` into `out` through a temporary file; any failure leaves no `.etl` behind.
fn copy_trace(source: &Path, out: &Path, cap: u64) -> Result<(), Failure> {
    let mut pending = output::create(out, "boot-traces", "etl").map_err(|_| Failure::Refused)?;
    let mut trace = open_source(source, cap)?;
    copy_capped(&mut trace, &mut *pending, cap)?;
    pending.commit().map_err(|_| Failure::WriteFailed)
}

/// Opens `path` read-only, letting its other users read, write and delete it, and refuses what is
/// not a regular file of at most `cap` bytes: a link, a folder or anything bigger.
fn open_source(path: &Path, cap: u64) -> Result<File, Failure> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => Failure::Missing,
            _ => Failure::Refused,
        })?;
    let meta = file.metadata().map_err(|_| Failure::Refused)?;
    let attributes = meta.file_attributes();
    let plain = attributes & (FILE_ATTRIBUTE_REPARSE_POINT.0 | FILE_ATTRIBUTE_DIRECTORY.0) == 0;
    if plain && meta.is_file() && meta.len() <= cap {
        Ok(file)
    } else {
        Err(Failure::Refused)
    }
}

/// Copies `from` to `to` in chunks; more than `cap` bytes is refused, so a file that grows while
/// it is read still stops at the limit.
fn copy_capped(from: &mut impl Read, to: &mut impl Write, cap: u64) -> Result<u64, Failure> {
    let mut buffer = vec![0u8; CHUNK];
    let mut total = 0u64;
    loop {
        let read = from.read(&mut buffer).map_err(|_| Failure::WriteFailed)?;
        if read == 0 {
            return Ok(total);
        }
        total += read as u64;
        if total > cap {
            return Err(Failure::Refused);
        }
        to.write_all(&buffer[..read])
            .map_err(|_| Failure::WriteFailed)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(dir: &tempfile::TempDir) -> PathBuf {
        let folder = dir
            .path()
            .join("Vestigium")
            .join("data")
            .join("boot-traces");
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn leftovers(folder: &Path) -> Vec<String> {
        std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_source_is_the_boot_logger_under_the_system_directory() {
        assert_eq!(
            source_path(Path::new(r"C:\Windows\System32")),
            PathBuf::from(r"C:\Windows\System32\WDI\LogFiles\BootPerfDiagLogger.etl")
        );
    }

    #[test]
    fn the_output_has_the_shape_of_a_boot_trace() {
        let good = r"C:\Users\someone\AppData\Local\Vestigium\data\boot-traces\1793580000000.etl";
        assert_eq!(
            output::check_shape(Path::new(good), "boot-traces", "etl"),
            Ok(())
        );
        for bad in [
            r"C:\Users\someone\AppData\Local\Vestigium\data\boot-traces\1.scan",
            r"C:\Users\someone\AppData\Local\Vestigium\data\boot-traces\1.etl.tmp",
            r"C:\Users\someone\AppData\Local\Vestigium\data\boot-traces\name.etl",
            r"C:\Users\someone\AppData\Local\Vestigium\data\scans\1.etl",
            r"C:\Users\someone\AppData\Local\Vestigium\data\boot-traces\..\1.etl",
            r"C:\Windows\System32\WDI\LogFiles\1.etl",
            r"\\server\share\Vestigium\data\boot-traces\1.etl",
        ] {
            assert!(
                output::check_shape(Path::new(bad), "boot-traces", "etl").is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_trace_is_copied_whole_across_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let folder = folder(&dir);
        let source = dir.path().join("trace.etl");
        let body: Vec<u8> = (0..CHUNK * 2 + 123).map(|i| (i % 251) as u8).collect();
        std::fs::write(&source, &body).unwrap();
        let out = folder.join("5.etl");
        assert_eq!(copy_trace(&source, &out, SOURCE_MAX_BYTES), Ok(()));
        assert_eq!(std::fs::read(&out).unwrap(), body);
        assert_eq!(leftovers(&folder), ["5.etl"]);
    }

    #[test]
    fn a_missing_trace_is_told_apart_and_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let folder = folder(&dir);
        let out = folder.join("6.etl");
        let source = dir.path().join("none.etl");
        assert_eq!(
            copy_trace(&source, &out, SOURCE_MAX_BYTES),
            Err(Failure::Missing)
        );
        assert_eq!(
            copy_trace(&dir.path().join("no folder").join("x.etl"), &out, 1),
            Err(Failure::Missing)
        );
        assert!(leftovers(&folder).is_empty());
    }

    #[test]
    fn a_folder_is_refused_as_a_source() {
        let dir = tempfile::tempdir().unwrap();
        let folder = folder(&dir);
        let out = folder.join("7.etl");
        assert_eq!(
            copy_trace(dir.path(), &out, SOURCE_MAX_BYTES),
            Err(Failure::Refused)
        );
        assert!(leftovers(&folder).is_empty());
    }

    #[test]
    fn a_file_over_the_cap_is_refused_and_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let folder = folder(&dir);
        let source = dir.path().join("big.etl");
        std::fs::write(&source, [1u8; 101]).unwrap();
        let out = folder.join("8.etl");
        assert_eq!(copy_trace(&source, &out, 100), Err(Failure::Refused));
        assert!(leftovers(&folder).is_empty());
        assert_eq!(copy_trace(&source, &out, 101), Ok(()));
    }

    #[test]
    fn a_file_that_outgrows_the_cap_while_read_stops_at_the_cap() {
        let mut from: &[u8] = &[7u8; 300];
        let mut to = Vec::new();
        assert_eq!(copy_capped(&mut from, &mut to, 299), Err(Failure::Refused));
        let mut from: &[u8] = &[7u8; 300];
        let mut to = Vec::new();
        assert_eq!(copy_capped(&mut from, &mut to, 300), Ok(300));
        assert_eq!(to.len(), 300);
    }

    #[test]
    fn a_link_is_refused_as_a_source() {
        let dir = tempfile::tempdir().unwrap();
        let folder = folder(&dir);
        let real = dir.path().join("real.etl");
        std::fs::write(&real, b"trace").unwrap();
        let link = dir.path().join("link.etl");
        if std::os::windows::fs::symlink_file(&real, &link).is_err() {
            eprintln!("no permission to make a symbolic link; the check is skipped");
            return;
        }
        let out = folder.join("9.etl");
        assert_eq!(
            copy_trace(&link, &out, SOURCE_MAX_BYTES),
            Err(Failure::Refused)
        );
        assert!(leftovers(&folder).is_empty());
    }

    #[test]
    fn a_source_other_programs_hold_open_is_still_read() {
        let dir = tempfile::tempdir().unwrap();
        let folder = folder(&dir);
        let source = dir.path().join("held.etl");
        let mut writer = OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0)
            .open(&source)
            .unwrap();
        writer.write_all(b"being written").unwrap();
        let out = folder.join("10.etl");
        assert_eq!(copy_trace(&source, &out, SOURCE_MAX_BYTES), Ok(()));
        assert_eq!(std::fs::read(&out).unwrap(), b"being written");
    }

    #[test]
    fn a_bad_output_is_refused_before_the_source_is_touched() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("trace.etl");
        std::fs::write(&source, b"trace").unwrap();
        let wrong = dir.path().join("trace-copy.etl");
        assert_eq!(
            copy_trace(&source, &wrong, SOURCE_MAX_BYTES),
            Err(Failure::Refused)
        );
        assert!(!wrong.exists());
    }

    #[test]
    fn an_existing_output_is_never_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let folder = folder(&dir);
        let source = dir.path().join("trace.etl");
        std::fs::write(&source, b"new").unwrap();
        let out = folder.join("11.etl");
        std::fs::write(&out, b"old").unwrap();
        assert_eq!(
            copy_trace(&source, &out, SOURCE_MAX_BYTES),
            Err(Failure::Refused)
        );
        assert_eq!(std::fs::read(&out).unwrap(), b"old");
    }
}
