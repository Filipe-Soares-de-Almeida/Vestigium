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
