//! Proves the elevated helper does not load a DLL planted beside it.
//!
//! The helper's manifest demands administrator, so the test is ignored by default. Build the helper,
//! then run it from an elevated shell:
//! `cargo build -p vestigium-helper && cargo test -p vestigium-helper dll_search -- --ignored`

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use windows::Win32::System::Diagnostics::Debug::{SEM_FAILCRITICALERRORS, SetErrorMode};

use crate::job;

/// System DLLs the helper imports that are not known DLLs.
const PLANTED: [&str; 10] = [
    "dbgeng.dll",
    "dbghelp.dll",
    "wevtapi.dll",
    "powrprof.dll",
    "bcrypt.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "dbgcore.dll",
    "dbgmodel.dll",
    "msvcp140.dll",
];

/// The helper the workspace build produced, found beside the test's own `deps` folder.
fn built_helper() -> PathBuf {
    let exe = std::env::current_exe().expect("the test executable has a path");
    let profile = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("the test executable sits in target/<profile>/deps");
    let helper = profile.join("vestigium-helper.exe");
    assert!(
        helper.is_file(),
        "build the helper first: cargo build -p vestigium-helper"
    );
    helper
}

#[test]
#[ignore = "needs an elevated shell and a built helper"]
fn planted_dlls_beside_the_helper_are_not_loaded() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let copy = dir.path().join("vestigium-helper.exe");
    std::fs::copy(built_helper(), &copy).expect("copy the helper");
    for name in PLANTED {
        std::fs::write(dir.path().join(name), b"not a library").expect("plant a file");
    }
    let missing = dir.path().join("1.plan");

    // A failed load would otherwise raise a dialog on the desktop and wait for a click.
    // SAFETY: setting the process error mode has no preconditions; children inherit it.
    unsafe { SetErrorMode(SEM_FAILCRITICALERRORS) };
    let mut child = Command::new(&copy)
        .args(["tweak", "--plan"])
        .arg(&missing)
        .spawn()
        .expect("start the helper from an elevated shell");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for the helper") {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            panic!("the helper did not finish");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    assert_eq!(
        status.code(),
        Some(i32::from(job::REFUSED)),
        "the helper loaded a planted file or failed to start: {status:?}"
    );
}
