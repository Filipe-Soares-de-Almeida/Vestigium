use vestigium_collect::boot_report;

#[test]
fn boot_report_reads_or_reports_the_missing_elevation() {
    match boot_report(30) {
        Ok(report) => assert!(report.boots.iter().all(|b| b.boot_ms > 0)),
        Err(err) => assert!(!err.to_string().is_empty()),
    }
}
