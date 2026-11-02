//! Vestigium's elevated helper: runs one job from a closed list and exits.
#![windows_subsystem = "windows"]

use std::process::ExitCode;

use vestigium_helper::{capture, job};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Ok(job) = job::parse(&args) else {
        return ExitCode::from(job::REFUSED);
    };
    match job {
        job::Job::DeepCapture { minutes, out, etl } => capture::run(minutes, &out, etl.as_deref()),
    }
}
