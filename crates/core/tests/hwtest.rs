use vestigium_core::health::Subject;
use vestigium_core::hwtest::{
    BANDS, BenchMode, BenchResult, Bus, DiagnosticResult, Expected, MemoryQuick, Outcome,
    QUICK_CHUNK, Region, SURFACE_REGIONS, SurfaceAggregator, TestData, TestRun, VolumeCheck,
    VolumeResult, bench_has_room, bench_reference, bench_verdict, caused_by_test, combined_surface,
    disk_subject, latency_band, memory_diagnostics, quick_test_size, volume_result,
};
use vestigium_core::{
    Channel, Culprit, DiskInfo, Episode, EpisodeKind, EventMeaning, Media, WinEvent,
};

const GIB: u64 = 1 << 30;

fn memdiag(ts_ms: i64, record_id: u64, id: u16, data_xml: &str) -> WinEvent {
    WinEvent {
        ts_ms,
        channel: Channel::System,
        provider: "Microsoft-Windows-MemoryDiagnostics-Results".into(),
        id,
        record_id,
        level: 4,
        meaning: EventMeaning::MemoryDiagnostic,
        data_xml: data_xml.into(),
    }
}

fn detail_xml(tested: u64, untested: u64, bad: u64) -> String {
    format!(
        "<UserData><Results><LaunchType>Manual</LaunchType><CompletionType>Pass</CompletionType>\
         <MemorySize>32713</MemorySize><TestType>10</TestType><TestDuration>2426</TestDuration>\
         <TestCount>12</TestCount><NumPagesTested>{tested}</NumPagesTested>\
         <NumPagesUnTested>{untested}</NumPagesUnTested><NumBadPages>{bad}</NumBadPages>\
         </Results></UserData>"
    )
}

#[test]
fn pairs_1101_and_1201_into_one_run() {
    let summary = "<UserData><Results><CompletionType>Pass</CompletionType></Results></UserData>";
    let runs = memory_diagnostics(&[
        memdiag(1_000, 11, 1201, summary),
        memdiag(1_000, 10, 1101, &detail_xml(8_319_731, 1_843, 0)),
        memdiag(90 * 86_400_000, 50, 1103, ""),
    ]);
    assert_eq!(runs.len(), 2);
    assert_eq!(
        (runs[0].record_id, runs[1].record_id),
        (10, 50),
        "named by the first event"
    );
    assert_eq!(runs[0].result, DiagnosticResult::Pass);
    let detail = runs[0].detail.expect("1101 carries the detail");
    assert_eq!(
        (detail.pages_tested, detail.duration_s, detail.test_type),
        (8_319_731, 2426, 10)
    );
    assert_eq!(runs[1].result, DiagnosticResult::Cancelled);
    assert_eq!(runs[1].detail, None);
}

#[test]
fn bad_pages_make_a_run_an_error_even_when_the_id_says_pass() {
    let runs = memory_diagnostics(&[memdiag(1_000, 1, 1101, &detail_xml(1000, 0, 3))]);
    assert_eq!(runs[0].result, DiagnosticResult::Errors);
}

#[test]
fn untested_pages_count_against_coverage() {
    let runs = memory_diagnostics(&[memdiag(1_000, 1, 1101, &detail_xml(750, 250, 0))]);
    let coverage = runs[0].detail.unwrap().coverage();
    assert!((coverage - 0.75).abs() < 1e-9);
}

#[test]
fn quick_test_keeps_use_below_80() {
    let total = 32 * GIB;
    let available = 21 * GIB;
    let size = quick_test_size(total, available).expect("room to test");
    let in_use_after = (total - available + size) as f64 / total as f64;
    assert!(in_use_after <= 0.80, "use would reach {in_use_after}");
    assert!(size <= available / 2);
    assert_eq!(size % QUICK_CHUNK, 0);

    let busy = quick_test_size(32 * GIB, 8 * GIB).unwrap();
    assert!((32 * GIB - 8 * GIB + busy) as f64 / (32 * GIB) as f64 <= 0.80);
}

#[test]
fn refuses_a_quick_test_with_little_free_memory() {
    assert_eq!(quick_test_size(8 * GIB, 2 * GIB), None);
    assert_eq!(quick_test_size(16 * GIB, 3 * GIB + GIB / 2), None);
}

#[test]
fn latency_bands_are_closed_on_the_left() {
    assert_eq!(latency_band(0.1), 0);
    assert_eq!(latency_band(5.0), 1);
    assert_eq!(latency_band(149.9), 3);
    assert_eq!(latency_band(500.0), BANDS - 1);
    assert_eq!(latency_band(30_000.0), BANDS - 1);
}

#[test]
fn surface_map_keeps_the_worst_block_of_each_region() {
    let disk = 1024 * (1 << 20);
    let mut scan = SurfaceAggregator::new(1, Media::Hdd, disk, 512, 0);
    scan.block(0, 4.0);
    scan.block(1 << 20, 80.0);
    scan.block(5 << 20, 2.0);
    scan.unreadable(9 << 20, &[(9 << 20) + 512]);
    let scan = scan.finish(disk, None);
    assert_eq!(scan.regions.len(), SURFACE_REGIONS);
    assert_eq!(scan.regions[1], Region::Read { worst_ms: 80.0 });
    assert_eq!(scan.regions[9], Region::Unreadable);
    assert_eq!(scan.regions[100], Region::Unread);
    assert_eq!(scan.worst[0].ms, 80.0);
    assert_eq!(scan.unreadable, vec![(9 << 20) + 512]);
    assert!(scan.whole_disk());
}

#[test]
fn resumed_scans_combine_into_one_view() {
    let disk = 100 << 20;
    let mut first = SurfaceAggregator::new(0, Media::Ssd, disk, 512, 0);
    first.block(0, 1.0);
    let first = first.finish(40 << 20, Some("stopped".into()));
    let mut second = SurfaceAggregator::new(0, Media::Ssd, disk, 512, 40 << 20);
    second.block(50 << 20, 600.0);
    let second = second.finish(disk, None);
    let mut restart = SurfaceAggregator::new(0, Media::Ssd, disk, 512, 0);
    restart.block(0, 1.0);
    let restart = restart.finish(10 << 20, None);

    let combined = combined_surface(&[first.clone(), second.clone()]).unwrap();
    assert!(combined.whole_disk());
    assert_eq!(combined.bands.iter().sum::<u64>(), 2);
    assert_eq!(combined.slow_blocks(), 1);

    let newest = combined_surface(&[first, second, restart]).unwrap();
    assert_eq!((newest.from_bytes, newest.to_bytes), (0, 10 << 20));
}

#[test]
fn benchmark_reference_follows_the_bus() {
    let sata = bench_reference(Bus::Sata, Media::Ssd, BenchMode::SeqRead).unwrap();
    let nvme = bench_reference(Bus::Nvme, Media::Ssd, BenchMode::SeqRead).unwrap();
    assert!(nvme.0 > sata.1);
    assert!(bench_reference(Bus::Usb, Media::Ssd, BenchMode::SeqRead).is_none());
    assert!(bench_reference(Bus::Sata, Media::Hdd, BenchMode::RandRead1).is_some());

    let slow = BenchResult {
        mode: BenchMode::SeqRead,
        mb_per_s: 231.0,
        iops: 231.0,
        max_ms: 72.0,
    };
    assert_eq!(
        bench_verdict(&slow, Bus::Sata, Media::Ssd),
        Some(Expected::Below)
    );
    assert_eq!(
        bench_verdict(&slow, Bus::Sata, Media::Hdd),
        Some(Expected::Inside)
    );
}

#[test]
fn chkdsk_exit_codes() {
    assert_eq!(volume_result(0, false), VolumeResult::NoProblems);
    assert_eq!(volume_result(2, false), VolumeResult::NoProblems);
    assert_eq!(volume_result(13, true), VolumeResult::ProblemsFound);
    assert_eq!(volume_result(13, false), VolumeResult::ProblemsFound);
    assert_eq!(volume_result(1, false), VolumeResult::ProblemsFound);
    assert_eq!(
        volume_result(5, false),
        VolumeResult::ProblemsFound,
        "fixed online"
    );
    assert_eq!(
        volume_result(3, false),
        VolumeResult::CouldNotCheck,
        "access denied, no volume"
    );
    assert_eq!(volume_result(3, true), VolumeResult::ProblemsFound);
    assert_eq!(volume_result(7, false), VolumeResult::CouldNotCheck);
}

#[test]
fn disk_subject_uses_the_counter_name() {
    let disks = [
        DiskInfo {
            name: "0 C:".into(),
            media: Media::Ssd,
        },
        DiskInfo {
            name: "1 D:".into(),
            media: Media::Hdd,
        },
    ];
    assert_eq!(disk_subject(1, &disks), Subject::Disk("1 D:".into()));
    assert_eq!(disk_subject(7, &disks), Subject::Disk("7".into()));
}

fn quick_run(started_ms: i64, ended_ms: i64) -> TestRun {
    TestRun {
        subject: Subject::Pc,
        started_ms,
        ended_ms,
        outcome: Outcome::Clean,
        data: TestData::MemoryQuick(MemoryQuick {
            total_bytes: 32 * GIB,
            tested_bytes: 4 * GIB,
            rounds: 3,
            passes: 21,
            error_count: 0,
            errors: Vec::new(),
            failure: None,
        }),
    }
}

fn episode(start_ms: i64, end_ms: i64, culprit: &str) -> Episode {
    Episode {
        kind: EpisodeKind::DiskBusy {
            disk: "0 C:".into(),
        },
        start_ms,
        end_ms,
        peak: 100.0,
        culprit: Some(Culprit {
            pid: 42,
            name: culprit.into(),
            value: 1.0,
        }),
        truncated: false,
    }
}

fn disk_busy(start_ms: i64, end_ms: i64, disk: &str, culprit: &str) -> Episode {
    Episode {
        kind: EpisodeKind::DiskBusy { disk: disk.into() },
        ..episode(start_ms, end_ms, culprit)
    }
}

fn volume_run(subject: Subject, started_ms: i64, ended_ms: i64) -> TestRun {
    TestRun {
        subject,
        started_ms,
        ended_ms,
        outcome: Outcome::Clean,
        data: TestData::VolumeCheck(VolumeCheck {
            volume: r"D:\".into(),
            exit_code: Some(0),
            result: VolumeResult::NoProblems,
            output: String::new(),
            dirty_before: Some(false),
            dirty_after: Some(false),
        }),
    }
}

#[test]
fn a_disk_busy_during_a_surface_scan_of_that_disk_is_the_tests() {
    let runs = [volume_run(Subject::Disk("1 D:".into()), 1_000, 60_000)];
    assert!(caused_by_test(
        &disk_busy(30_000, 90_000, "1 D:", "chkdsk.exe"),
        &runs
    ));
    assert!(!caused_by_test(
        &disk_busy(70_000, 90_000, "1 D:", "chkdsk.exe"),
        &runs
    ));
}

#[test]
fn a_disk_busy_on_another_disk_is_not_the_tests() {
    let runs = [volume_run(Subject::Disk("0 C:".into()), 1_000, 60_000)];
    assert!(!caused_by_test(
        &disk_busy(30_000, 90_000, "1 D:", "chkdsk.exe"),
        &runs
    ));
}

#[test]
fn a_test_names_its_disk_by_number_or_by_drive_letter() {
    let busy = disk_busy(30_000, 90_000, "1 D: E:", "System");
    let by_number = [volume_run(Subject::Disk("1".into()), 1_000, 60_000)];
    let by_letter = [volume_run(Subject::Volume(r"E:\".into()), 1_000, 60_000)];
    let other_letter = [volume_run(Subject::Volume(r"C:\".into()), 1_000, 60_000)];

    assert!(caused_by_test(&busy, &by_number));
    assert!(caused_by_test(&busy, &by_letter));
    assert!(!caused_by_test(&busy, &other_letter));
}

#[test]
fn a_memory_test_does_not_claim_a_busy_disk() {
    let runs = [quick_run(1_000, 60_000)];
    assert!(!caused_by_test(
        &disk_busy(30_000, 90_000, "1 D:", "System"),
        &runs
    ));
}

#[test]
fn episode_by_vestigium_during_a_test_is_marked() {
    let runs = [quick_run(1_000, 60_000)];
    assert!(caused_by_test(
        &episode(30_000, 90_000, "Vestigium.Helper.exe"),
        &runs
    ));
    assert!(!caused_by_test(
        &episode(30_000, 90_000, "msedge.exe"),
        &runs
    ));
    assert!(!caused_by_test(
        &episode(70_000, 90_000, "Vestigium.Window.exe"),
        &runs
    ));
}

#[test]
fn the_benchmark_needs_its_file_and_a_margin_free() {
    let mib = 1u64 << 20;
    let free = |gib: f64| (gib * GIB as f64) as u64;
    assert!(!bench_has_room(free(1.4), GIB));
    assert!(!bench_has_room(GIB + 512 * mib - 1, GIB));
    assert!(bench_has_room(GIB + 512 * mib, GIB));
    assert!(bench_has_room(free(1.6), GIB));
    assert!(bench_has_room(16 * mib + 512 * mib, 16 * mib));
}
