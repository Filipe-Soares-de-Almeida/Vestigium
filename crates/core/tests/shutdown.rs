use vestigium_core::{PowerLossDetail, SHUTTING_DOWN, power_loss_detail, unexpected_shutdown_ms};

/// A Kernel-Power 41 `<EventData>` with the fields Windows 10 22H2 writes.
fn kernel_power(code: u32, param1: &str, power_state: &str, button: u64, long: bool) -> String {
    format!(
        "<EventData><Data Name='BugcheckCode'>{code}</Data>\
         <Data Name='BugcheckParameter1'>{param1}</Data>\
         <Data Name='BugcheckParameter2'>0x0</Data>\
         <Data Name='BugcheckParameter3'>0xfffff80000001234</Data>\
         <Data Name='BugcheckParameter4'>0x2</Data>\
         <Data Name='SleepInProgress'>{power_state}</Data>\
         <Data Name='PowerButtonTimestamp'>{button}</Data>\
         <Data Name='BootAppStatus'>0</Data>\
         <Data Name='LongPowerButtonPressDetected'>{long}</Data></EventData>"
    )
}

/// A 6008 `<EventData>` whose binary holds `local` then `utc` as SYSTEMTIMEs.
fn unexpected_shutdown(local: [u16; 8], utc: [u16; 8]) -> String {
    let hex: String = local
        .iter()
        .chain(utc.iter())
        .flat_map(|v| v.to_le_bytes())
        .map(|b| format!("{b:02X}"))
        .collect();
    format!(
        "<EventData><Data>08:30:00</Data><Data>\u{200e}15/\u{200e}01/\u{200e}2030</Data>\
         <Data></Data><Data></Data><Data>3600</Data><Data></Data><Data></Data>\
         <Binary>{hex}600900003C000000</Binary></EventData>"
    )
}

#[test]
fn blue_screen_keeps_its_code_and_parameters() {
    let detail = power_loss_detail(&kernel_power(159, "0x3", "0", 0, false)).unwrap();

    assert_eq!(
        detail,
        PowerLossDetail {
            bugcheck_code: 0x9f,
            bugcheck_params: [3, 0, 0xffff_f800_0000_1234, 2],
            power_state: 0,
            power_button: false,
        }
    );
}

#[test]
fn power_button_counts_either_field() {
    let stamped = power_loss_detail(&kernel_power(0, "0x0", "0", 131_728_546_170_882_432, false));
    let long = power_loss_detail(&kernel_power(0, "0x0", "0", 0, true));

    assert!(stamped.unwrap().power_button);
    assert!(long.unwrap().power_button);
}

#[test]
fn sleep_in_progress_is_a_power_state() {
    let detail = power_loss_detail(&kernel_power(0, "0x0", "6", 0, false)).unwrap();

    assert_eq!(detail.power_state, SHUTTING_DOWN);
    assert!(!detail.power_button);
}

#[test]
fn all_zero_is_a_crash_with_no_trace() {
    let detail = power_loss_detail(&kernel_power(0, "0x0", "0", 0, false)).unwrap();

    assert_eq!(detail.bugcheck_code, 0);
    assert_eq!(detail.power_state, 0);
    assert!(!detail.power_button);
}

#[test]
fn double_quoted_names_parse_too() {
    let xml = kernel_power(10, "0x8", "4", 0, false).replace('\'', "\"");

    let detail = power_loss_detail(&xml).unwrap();

    assert_eq!(detail.bugcheck_code, 10);
    assert_eq!(detail.bugcheck_params[0], 8);
    assert_eq!(detail.power_state, 4);
}

#[test]
fn old_boolean_sleep_flag_reads_as_no_state() {
    let detail = power_loss_detail(&kernel_power(0, "0x0", "false", 0, false)).unwrap();

    assert_eq!(detail.power_state, 0);
}

#[test]
fn event_without_a_stop_code_has_no_detail() {
    assert_eq!(power_loss_detail("<EventData></EventData>"), None);
    assert_eq!(power_loss_detail(""), None);
    assert_eq!(
        power_loss_detail("<EventData><Data Name='BugcheckCode'>soon</Data></EventData>"),
        None
    );
}

#[test]
fn unexpected_shutdown_reads_the_utc_time() {
    let xml = unexpected_shutdown(
        [2030, 1, 2, 15, 8, 30, 0, 250],
        [2030, 1, 2, 15, 11, 30, 0, 250],
    );

    // 2030-01-15T11:30:00.250Z
    assert_eq!(unexpected_shutdown_ms(&xml), Some(1_894_707_000_250));
}

#[test]
fn unexpected_shutdown_handles_leap_days_and_month_ends() {
    let xml = unexpected_shutdown(
        [2028, 2, 2, 29, 0, 0, 0, 0],
        [2028, 2, 2, 29, 23, 59, 59, 999],
    );

    // 2028-02-29T23:59:59.999Z
    assert_eq!(unexpected_shutdown_ms(&xml), Some(1_835_481_599_999));
}

#[test]
fn broken_binary_has_no_time() {
    assert_eq!(
        unexpected_shutdown_ms("<EventData><Binary>EA07</Binary></EventData>"),
        None
    );
    assert_eq!(
        unexpected_shutdown_ms("<EventData><Binary>EA0</Binary></EventData>"),
        None
    );
    assert_eq!(
        unexpected_shutdown_ms("<EventData><Binary>ZZ</Binary></EventData>"),
        None
    );
    assert_eq!(unexpected_shutdown_ms("<EventData></EventData>"), None);
    let month_13 = unexpected_shutdown([2030, 13, 0, 1, 0, 0, 0, 0], [2030, 13, 0, 1, 0, 0, 0, 0]);
    assert_eq!(unexpected_shutdown_ms(&month_13), None);
}
