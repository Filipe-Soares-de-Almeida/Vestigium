//! Vestigium's elevated helper: runs one job from a closed list and exits.
#![windows_subsystem = "windows"]

use std::process::ExitCode;

use windows::Win32::System::LibraryLoader::{
    LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
};

use vestigium_helper::{capture, chkdsk, delete, dumps, job, scan, surface, tweak};

fn main() -> ExitCode {
    // Libraries loaded from here on come from System32 only; a failure leaves the build-time limit.
    // SAFETY: called once at startup, before any thread or library load of the helper's own.
    let _ = unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32) };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Ok(job) = job::parse(&args) else {
        return ExitCode::from(job::REFUSED);
    };
    match job {
        job::Job::DeepCapture { minutes, out } => capture::run(minutes, &out),
        job::Job::HealthScan { out } => scan::run(&out),
        job::Job::DumpScan { out } => dumps::run(&out),
        job::Job::DeleteDumps { list } => delete::run(&list),
        job::Job::Tweak { plan, sha256 } => tweak::run(&plan, &sha256),
        job::Job::DiskScan {
            disk,
            from_mib,
            out,
        } => surface::run(disk, from_mib, &out),
        job::Job::VolumeCheck { letter, out } => chkdsk::run(letter, &out),
    }
}
