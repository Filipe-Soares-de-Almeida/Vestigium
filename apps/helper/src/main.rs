//! Vestigium's elevated helper: runs one job from a closed list and exits.
#![windows_subsystem = "windows"]

use std::process::ExitCode;

use vestigium_helper::{capture, chkdsk, delete, dumps, job, scan, surface, tweak};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Ok(job) = job::parse(&args) else {
        return ExitCode::from(job::REFUSED);
    };
    match job {
        job::Job::DeepCapture { minutes, out, etl } => capture::run(minutes, &out, etl.as_deref()),
        job::Job::HealthScan { out } => scan::run(&out),
        job::Job::DumpScan { out } => dumps::run(&out),
        job::Job::DeleteDumps { list } => delete::run(&list),
        job::Job::Tweak { plan } => tweak::run(&plan),
        job::Job::DiskScan {
            disk,
            from_mib,
            out,
        } => surface::run(disk, from_mib, &out),
        job::Job::VolumeCheck { letter, out } => chkdsk::run(letter, &out),
    }
}
