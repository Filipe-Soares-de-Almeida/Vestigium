use vestigium_collect::boot_report;

#[test]
fn boot_report_reads_or_reports_the_missing_elevation() {
    match boot_report(30) {
        Ok(report) => assert!(report.boots.iter().all(|b| b.boot_ms > 0)),
        Err(err) => assert!(!err.to_string().is_empty()),
    }
}

#[test]
fn boot_timeline_reads_without_elevation() {
    let boots =
        vestigium_collect::boot_timeline(30).expect("the System log is readable by any user");
    // A machine always has its current boot; earlier ones depend on how long it has been running.
    let last = boots.last().expect("the current boot");
    let p = last.phases();
    assert!(last.kernel_start_ms > 0);
    for pair in boots.windows(2) {
        assert!(pair[0].kernel_start_ms < pair[1].kernel_start_ms);
    }
    if let (Some(signin), Some(profile)) = (p.to_signin_ms, p.profile_ms) {
        assert!(signin < 86_400_000 && profile < 86_400_000);
    }
}
