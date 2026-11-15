use vestigium_core::health::{BootLog, BootSource};

const T0: i64 = 1_794_000_000_000;

fn data(fields: &[(&str, &str)]) -> String {
    let inner: String = fields
        .iter()
        .map(|(name, value)| format!("<Data Name='{name}'>{value}</Data>"))
        .collect();
    format!("<EventData>{inner}</EventData>")
}

/// A full boot shaped like a slow sign-in: the profile step outlasts the TPM check.
fn slow_boot(log: &mut BootLog, at: i64) {
    let ok = [
        log.add(BootSource::KernelBoot, 27, at, &data(&[("BootType", "0")])),
        log.add(
            BootSource::KernelBoot,
            30,
            at,
            &data(&[("ExitBootServicesExit", "15215")]),
        ),
        log.add(
            BootSource::KernelGeneral,
            12,
            at,
            &data(&[("StartTime", "2026-11-06T21:20:00.0000000Z")]),
        ),
        log.add(
            BootSource::Winlogon,
            7001,
            at + 19_449,
            &data(&[("TSId", "1")]),
        ),
        log.add(
            BootSource::UserProfile,
            1,
            at + 19_454,
            &data(&[("Session", "1")]),
        ),
        log.add(BootSource::Tpm, 18, at + 23_000, ""),
        log.add(BootSource::Tpm, 1025, at + 55_890, ""),
        log.add(
            BootSource::UserProfile,
            2,
            at + 171_749,
            &data(&[("Session", "1")]),
        ),
        log.add(
            BootSource::ShellCore,
            62171,
            at + 172_000,
            &data(&[("TaskName", "UndockedShellUpdate")]),
        ),
        log.add(
            BootSource::ShellCore,
            62171,
            at + 178_000,
            &data(&[("TaskName", "AllLogonTasks")]),
        ),
        log.add(
            BootSource::ShellCore,
            9707,
            at + 188_000,
            &data(&[("Command", "Example.exe")]),
        ),
        log.add(
            BootSource::ShellCore,
            9708,
            at + 190_500,
            &data(&[("PID", "1"), ("Command", "Example.exe")]),
        ),
        log.add(
            BootSource::ServiceControl,
            7026,
            at + 24_000,
            &data(&[("param1", "\ndam\n")]),
        ),
    ];
    // The second Shell-Core 62171 names another logon task and is not kept.
    assert_eq!(
        ok,
        [
            true, true, true, true, true, true, true, true, false, true, true, true, true
        ]
    );
}

#[test]
fn a_full_boot_folds_into_its_phases() {
    let mut log = BootLog::default();
    slow_boot(&mut log, T0);
    let boots = log.boots();
    assert_eq!(boots.len(), 1);
    let boot = &boots[0];
    assert_eq!(boot.kernel_start_ms, T0);
    assert_eq!(boot.boot_type, Some(0));
    assert_eq!(boot.failed_drivers, ["dam"]);
    assert_eq!(boot.apps.len(), 1);
    assert_eq!(boot.apps[0].end_ms, Some(T0 + 190_500));

    let p = boot.phases();
    assert_eq!(p.firmware_ms, Some(15_215));
    assert_eq!(p.to_signin_ms, Some(19_449));
    assert!(!p.probable_wait);
    assert_eq!(p.profile_ms, Some(152_295));
    assert_eq!(p.tpm_wait_ms, Some(36_436));
    assert_eq!(p.desktop_ms, Some(6_251));
    assert_eq!(p.to_desktop_ms, Some(178_000));
    assert_eq!(p.from_power_ms, Some(193_215));
}

#[test]
fn a_resume_with_no_sign_in_has_no_phases_after_the_firmware() {
    let mut log = BootLog::default();
    slow_boot(&mut log, T0);
    let resume = T0 + 3_600_000;
    assert!(log.add(
        BootSource::KernelBoot,
        27,
        resume,
        &data(&[("BootType", "2")])
    ));
    assert!(log.add(
        BootSource::KernelBoot,
        30,
        resume,
        &data(&[("ExitBootServicesExit", "9000")])
    ));
    let boots = log.boots();
    assert_eq!(boots.len(), 2);
    let p = boots[1].phases();
    assert_eq!(boots[1].kernel_start_ms, resume);
    assert_eq!(boots[1].boot_type, Some(2));
    assert_eq!(p.firmware_ms, Some(9_000));
    assert_eq!(
        (
            p.to_signin_ms,
            p.profile_ms,
            p.to_desktop_ms,
            p.from_power_ms
        ),
        (None, None, None, None)
    );
}

#[test]
fn a_long_wait_before_sign_in_is_a_probable_wait() {
    let mut log = BootLog::default();
    log.add(BootSource::KernelBoot, 27, T0, &data(&[("BootType", "0")]));
    log.add(BootSource::Winlogon, 7001, T0 + 708_000, "");
    let p = log.boots()[0].phases();
    assert_eq!(p.to_signin_ms, Some(708_000));
    assert!(p.probable_wait);
}

#[test]
fn a_tpm_ready_before_sign_in_is_no_wait() {
    let mut log = BootLog::default();
    log.add(BootSource::KernelBoot, 27, T0, &data(&[("BootType", "0")]));
    log.add(BootSource::Tpm, 1025, T0 + 10_000, "");
    log.add(BootSource::UserProfile, 1, T0 + 20_000, "");
    log.add(BootSource::UserProfile, 2, T0 + 28_000, "");
    let p = log.boots()[0].phases();
    assert_eq!(p.profile_ms, Some(8_000));
    assert_eq!(p.tpm_wait_ms, Some(0));
}

#[test]
fn only_the_boot_logger_filling_up_is_kept() {
    let mut log = BootLog::default();
    log.add(BootSource::KernelBoot, 27, T0, &data(&[("BootType", "0")]));
    assert!(!log.add(
        BootSource::EventTracing,
        4,
        T0 + 1,
        &data(&[("SessionName", "Other Session")])
    ));
    assert!(log.add(
        BootSource::EventTracing,
        4,
        T0 + 2,
        &data(&[("SessionName", "PerfDiag Logger")])
    ));
    assert!(log.boots()[0].logger_full);
}
