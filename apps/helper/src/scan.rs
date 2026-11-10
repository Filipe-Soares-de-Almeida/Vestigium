use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use vestigium_collect::{boot_report, wall_ms};
use vestigium_core::health::{SCAN_VERSION, ScanResult};

use crate::{job, output};

/// Days of boot diagnostics a scan reads.
pub const SCAN_DAYS: u32 = 30;

/// Reads what needs elevation for the health page and writes it to `out`; a log it cannot read
/// leaves its part empty rather than failing the scan.
pub fn run(out: &Path) -> ExitCode {
    let Ok(mut file) = output::create(out, "scans", "scan") else {
        return ExitCode::from(job::REFUSED);
    };
    let scan = ScanResult {
        ts_ms: wall_ms(),
        boot: boot_report(SCAN_DAYS).ok(),
    };
    let mut bytes = vec![SCAN_VERSION];
    match postcard::to_allocvec(&scan) {
        Ok(body) => bytes.extend(body),
        Err(_) => return ExitCode::from(job::WRITE_FAILED),
    }
    if file.write_all(&bytes).is_err() || file.commit().is_err() {
        return ExitCode::from(job::WRITE_FAILED);
    }
    ExitCode::SUCCESS
}
