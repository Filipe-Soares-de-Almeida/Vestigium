use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use vestigium_collect::{image_path, run_as, signature};
use vestigium_core::{RunAs, Signature};

#[test]
fn own_process_reads_its_path_and_user() {
    let path = image_path(std::process::id()).expect("a process can name its own file");
    let exe = std::env::current_exe().unwrap();
    assert_eq!(Path::new(&path).file_name(), exe.file_name());
    assert!(matches!(
        run_as(std::process::id()),
        RunAs::CurrentUser { .. }
    ));
}

#[test]
fn a_build_output_is_unsigned() {
    assert_eq!(
        signature(&std::env::current_exe().unwrap()),
        Signature::Unsigned
    );
}

#[test]
fn a_windows_program_is_signed_through_its_catalog() {
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let notepad = Path::new(&windir).join("System32").join("notepad.exe");
    match signature(&notepad) {
        Signature::Valid { signer } => assert!(signer.contains("Microsoft"), "signed by {signer}"),
        other => panic!("notepad.exe reads as {other:?}"),
    }
}

#[test]
fn a_process_that_is_gone_is_unknown() {
    assert_eq!(image_path(u32::MAX - 3), None);
    assert_eq!(run_as(u32::MAX - 3), RunAs::Unknown);
}

/// Starts a copy of a small system program in a folder chain longer than the first path buffer.
/// Creates and removes files under the temp folder; run by hand with
/// `cargo test -p vestigium-collect --test process_info -- --ignored a_long_image_path`.
#[test]
#[ignore = "creates and removes a deep folder under the temp folder"]
fn a_long_image_path_is_read_in_full() {
    let root = std::env::temp_dir().join(format!("vestigium-long-path-{}", std::process::id()));
    let mut deep = std::path::PathBuf::from(format!(r"\\?\{}", root.display()));
    while deep.as_os_str().encode_wide().count() < 1_100 {
        deep.push("a-folder-with-a-fairly-long-name-to-fill-the-path");
    }
    std::fs::create_dir_all(&deep).unwrap_or_else(|e| panic!("{e}: {}", deep.display()));
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let program = deep.join("ping.exe");
    std::fs::copy(
        Path::new(&windir).join("System32").join("ping.exe"),
        &program,
    )
    .unwrap();

    let started = std::process::Command::new(&program)
        .args(["-n", "30", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .spawn();
    let read = started.as_ref().ok().map(|child| image_path(child.id()));
    if let Ok(mut child) = started {
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = std::fs::remove_dir_all(format!(r"\\?\{}", root.display()));

    let read = read.expect("the copy did not start");
    let path = read.expect("a long image path was lost");
    assert!(path.encode_utf16().count() > 1_024, "read {path}");
    assert!(path.ends_with("ping.exe"));
}
