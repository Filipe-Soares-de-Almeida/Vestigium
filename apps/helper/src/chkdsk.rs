use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use vestigium_collect::wall_ms;
use vestigium_core::health::Subject;
use vestigium_core::hwtest::{
    Outcome, TestData, TestRun, VolumeCheck, VolumeResult, volume_result,
};
use windows::Win32::Globalization::{CP_OEMCP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS, MultiByteToWideChar};
use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumeInformationW};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::System::Threading::{BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW};
use windows::core::HSTRING;

use crate::surface::write_run;
use crate::{job, output};

const DRIVE_FIXED: u32 = 3;
/// chkdsk's text kept for the technical level, from its end.
const OUTPUT_KEPT: usize = 16 * 1024;

/// Whether a volume may be checked: a local fixed disk holding NTFS.
pub fn check_volume(drive_type: u32, file_system: &str) -> Result<(), String> {
    if drive_type != DRIVE_FIXED {
        return Err("not a fixed local volume".into());
    }
    if !file_system.eq_ignore_ascii_case("NTFS") {
        return Err(format!("{file_system} is not NTFS"));
    }
    Ok(())
}

fn system_directory() -> Option<PathBuf> {
    let mut dir = [0u16; 260];
    // SAFETY: the buffer is writable for its whole length.
    let len = unsafe { GetSystemDirectoryW(Some(&mut dir)) } as usize;
    (len > 0 && len < dir.len()).then(|| PathBuf::from(String::from_utf16_lossy(&dir[..len])))
}

/// chkdsk from the system folder and its only arguments: the volume and an online, read-only scan.
pub fn chkdsk_command(system: &Path, letter: char) -> (PathBuf, [String; 2]) {
    (
        system.join("chkdsk.exe"),
        [format!("{letter}:"), "/scan".to_string()],
    )
}

fn oem_text(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    // SAFETY: a size query, then a conversion into a buffer of that size.
    unsafe {
        let len = MultiByteToWideChar(CP_OEMCP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, None);
        let mut wide = vec![0u16; len.max(0) as usize];
        let len = MultiByteToWideChar(
            CP_OEMCP,
            MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0),
            bytes,
            Some(&mut wide),
        );
        String::from_utf16_lossy(&wide[..len.max(0) as usize])
    }
}

fn tail(text: &str) -> String {
    let start = text.len().saturating_sub(OUTPUT_KEPT);
    let start = (start..=text.len())
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(text.len());
    text[start..].to_string()
}

fn volume_facts(letter: char) -> Result<(), String> {
    let root = HSTRING::from(format!("{letter}:\\"));
    // SAFETY: a NUL-terminated root path.
    let drive_type = unsafe { GetDriveTypeW(&root) };
    let mut fs = [0u16; 64];
    // SAFETY: the file system name buffer is writable for its whole length.
    unsafe { GetVolumeInformationW(&root, None, None, None, None, Some(&mut fs)) }
        .map_err(|e| e.to_string())?;
    let end = fs.iter().position(|&c| c == 0).unwrap_or(fs.len());
    check_volume(drive_type, &String::from_utf16_lossy(&fs[..end]))
}

/// Runs `chkdsk <letter>: /scan` at below-normal priority, without a window, and keeps its exit code
/// and the end of its text.
pub fn check(letter: char) -> (Outcome, VolumeCheck) {
    let mut found = VolumeCheck {
        volume: format!("{letter}:\\"),
        exit_code: None,
        result: VolumeResult::CouldNotCheck,
        output: String::new(),
    };
    if let Err(reason) = volume_facts(letter) {
        found.output = reason;
        return (Outcome::Failed, found);
    }
    let Some(system) = system_directory() else {
        return (Outcome::Failed, found);
    };
    let (exe, args) = chkdsk_command(&system, letter);
    let ran = Command::new(exe)
        .args(args)
        .current_dir(&system)
        .stdin(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW.0 | BELOW_NORMAL_PRIORITY_CLASS.0)
        .output();
    let ran = match ran {
        Ok(ran) => ran,
        Err(e) => {
            found.output = e.to_string();
            return (Outcome::Failed, found);
        }
    };
    found.output = tail(&oem_text(&ran.stdout));
    let Some(code) = ran.status.code().and_then(|c| u32::try_from(c).ok()) else {
        return (Outcome::Failed, found);
    };
    found.exit_code = Some(code);
    found.result = volume_result(code);
    let outcome = match found.result {
        VolumeResult::NoProblems => Outcome::Clean,
        VolumeResult::ProblemsFound => Outcome::Problems,
        VolumeResult::CouldNotCheck => Outcome::Failed,
    };
    (outcome, found)
}

/// Checks the file system of volume `letter` into a new `.chkdsk` file.
pub fn run(letter: char, out: &Path) -> ExitCode {
    let Ok(mut file) = output::create(out, "tests", "chkdsk") else {
        return ExitCode::from(job::REFUSED);
    };
    let started_ms = wall_ms();
    let (outcome, found) = check(letter);
    let run = TestRun {
        subject: Subject::Volume(found.volume.clone()),
        started_ms,
        ended_ms: wall_ms(),
        outcome,
        data: TestData::VolumeCheck(found),
    };
    if write_run(&mut file, &run) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(job::WRITE_FAILED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_a_non_ntfs_volume() {
        assert!(check_volume(DRIVE_FIXED, "NTFS").is_ok());
        assert!(check_volume(DRIVE_FIXED, "ntfs").is_ok());
        assert!(check_volume(DRIVE_FIXED, "FAT32").is_err());
        assert!(check_volume(DRIVE_FIXED, "ReFS").is_err());
        assert!(check_volume(2, "NTFS").is_err(), "removable");
        assert!(check_volume(4, "NTFS").is_err(), "network");
    }

    #[test]
    fn chkdsk_arguments_are_fixed() {
        let system = Path::new(r"C:\Windows\System32");
        let (exe, args) = chkdsk_command(system, 'D');
        assert_eq!(exe, PathBuf::from(r"C:\Windows\System32\chkdsk.exe"));
        assert_eq!(args, ["D:".to_string(), "/scan".to_string()]);
    }

    #[test]
    fn keeps_the_end_of_long_output() {
        let long = "é".repeat(OUTPUT_KEPT);
        let kept = tail(&long);
        assert!(kept.len() <= OUTPUT_KEPT);
        assert!(kept.chars().all(|c| c == 'é'));
    }

    #[test]
    fn output_outside_the_tests_folder_is_refused() {
        assert_eq!(
            run('C', Path::new(r"C:\Windows\Temp\1.chkdsk")),
            ExitCode::from(job::REFUSED)
        );
    }
}
