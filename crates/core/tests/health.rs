use vestigium_core::health::{
    BootDelay, BootReport, BootTime, Concern, DelayKind, DiskHealth, Finding, FindingChange,
    FindingKind, HealthFact, HealthInputs, Measure, NvmeHealth, PageFile, Reference,
    SecurityProduct, SmartAttribute, StoredFinding, Subject, VolumeSpace, evaluate, nvme_health,
    reconcile, smart_attributes,
};
use vestigium_core::{Channel, DiskInfo, EventMeaning, MachineInfo, Media, NextStep, WinEvent};

const DAY: i64 = 24 * 60 * 60_000;
const NOW: i64 = 100 * DAY;
const GB: u64 = 1 << 30;

fn healthy() -> HealthInputs {
    HealthInputs {
        now_ms: NOW,
        history_from_ms: 0,
        machine: MachineInfo {
            logical_cpus: 8,
            mem_total_bytes: 32 * GB,
            disks: vec![
                DiskInfo {
                    name: "0 C:".into(),
                    media: Media::Ssd,
                },
                DiskInfo {
                    name: "1 D:".into(),
                    media: Media::Hdd,
                },
            ],
        },
        system_disk: Some("0 C:".into()),
        disks: vec![disk(vec![attr(5, 0), attr(9, 4000), attr(197, 0)])],
        volumes: vec![volume(450, 900)],
        page_files: Some(vec![PageFile {
            path: r"C:\pagefile.sys".into(),
            size_bytes: 8 * GB,
            peak_bytes: GB,
        }]),
        commit_peak_pct_7d: Some(40.0),
        memory_high_days_7d: 0,
        events: Vec::new(),
        antivirus: Some(vec![product("Built-in antivirus", true)]),
        boot: None,
        open: Vec::new(),
    }
}

fn attr(id: u8, raw: u64) -> SmartAttribute {
    SmartAttribute {
        id,
        current: 100,
        worst: 100,
        raw,
    }
}

fn disk(attributes: Vec<SmartAttribute>) -> DiskHealth {
    DiskHealth {
        name: "0 C:".into(),
        read_ms: NOW,
        predict_failure: false,
        attributes,
        nvme: None,
    }
}

fn nvme(critical_warning: u8, percentage_used: u8, media_errors: u64) -> NvmeHealth {
    NvmeHealth {
        critical_warning,
        temperature_c: 40,
        available_spare_pct: 100,
        spare_threshold_pct: 10,
        percentage_used,
        media_errors,
    }
}

fn volume(free_gb: u64, total_gb: u64) -> VolumeSpace {
    VolumeSpace {
        root: r"C:\".into(),
        free_bytes: free_gb * GB,
        total_bytes: total_gb * GB,
        system: true,
    }
}

fn product(name: &str, on: bool) -> SecurityProduct {
    SecurityProduct {
        name: name.into(),
        on,
    }
}

fn event(days_ago: i64, provider: &str, id: u16, meaning: EventMeaning, data: &str) -> WinEvent {
    WinEvent {
        ts_ms: NOW - days_ago * DAY,
        channel: Channel::System,
        provider: provider.into(),
        id,
        record_id: (days_ago * 1000 + i64::from(id)) as u64,
        level: 2,
        meaning,
        data_xml: data.into(),
    }
}

fn power_loss(days_ago: i64, bugcheck: u32, power_state: u8) -> WinEvent {
    let data = format!(
        "<EventData><Data Name='BugcheckCode'>{bugcheck}</Data>\
         <Data Name='SleepInProgress'>{power_state}</Data>\
         <Data Name='PowerButtonTimestamp'>0</Data></EventData>"
    );
    event(
        days_ago,
        "Microsoft-Windows-Kernel-Power",
        41,
        EventMeaning::PowerLoss,
        &data,
    )
}

fn app_crash(days_ago: i64) -> WinEvent {
    let mut e = event(
        days_ago,
        "Application Error",
        1000,
        EventMeaning::AppCrash,
        "",
    );
    e.channel = Channel::Application;
    e
}

fn find(found: &[Finding], kind: FindingKind) -> Option<&Finding> {
    found.iter().find(|f| f.kind == kind)
}

#[test]
fn healthy_pc_has_no_findings() {
    assert_eq!(evaluate(&healthy()), Vec::new());
}

#[test]
fn smart_raw_counters_raise_attention() {
    let mut inputs = healthy();
    inputs.disks = vec![disk(vec![attr(187, 1), attr(9, 4000)])];

    let found = evaluate(&inputs);
    let smart = find(&found, FindingKind::DiskSmart).unwrap();

    assert_eq!(smart.concern, Concern::Attention);
    assert_eq!(smart.subject, Subject::Disk("0 C:".into()));
    assert_eq!(smart.value, Measure::Count(1));
    assert_eq!(smart.evidence, vec![HealthFact::Attribute(attr(187, 1))]);
    assert_eq!(
        smart.next_steps,
        [NextStep::BackUpNow, NextStep::RunDiskTest]
    );
}

#[test]
fn predict_failure_is_critical() {
    let mut inputs = healthy();
    inputs.disks[0].predict_failure = true;

    let found = evaluate(&inputs);
    let smart = find(&found, FindingKind::DiskSmart).unwrap();

    assert_eq!(smart.concern, Concern::Critical);
    assert!(smart.evidence.contains(&HealthFact::PredictFailure));
    assert!(smart.next_steps.contains(&NextStep::ReplaceDisk));
}

#[test]
fn nvme_critical_warning_is_critical_and_wear_is_attention() {
    let mut inputs = healthy();
    inputs.disks[0].nvme = Some(nvme(0b100, 12, 0));
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::DiskSmart)
            .unwrap()
            .concern,
        Concern::Critical
    );

    inputs.disks[0].nvme = Some(nvme(0, 93, 0));
    let found = evaluate(&inputs);
    let worn = find(&found, FindingKind::DiskSmart).unwrap();
    assert_eq!(worn.concern, Concern::Attention);
    assert_eq!(worn.value, Measure::Percent(93.0));

    inputs.disks[0].nvme = Some(nvme(0, 12, 0));
    assert_eq!(find(&evaluate(&inputs), FindingKind::DiskSmart), None);
}

#[test]
fn smart_parsers_read_the_blocks_and_survive_garbage() {
    let mut block = vec![0u8; 512];
    block[2..14].copy_from_slice(&[5, 0x33, 0, 100, 99, 7, 0, 0, 0, 0, 0, 0]);
    block[14..26].copy_from_slice(&[194, 0x22, 0, 36, 60, 40, 0, 20, 0, 60, 0, 0]);

    let attributes = smart_attributes(&block);

    assert_eq!(attributes.len(), 2);
    assert_eq!(
        attributes[0],
        SmartAttribute {
            id: 5,
            current: 100,
            worst: 99,
            raw: 7
        }
    );
    assert_eq!(smart_attributes(&[0; 10]), Vec::new());
    assert_eq!(smart_attributes(&[]), Vec::new());

    let mut log = vec![0u8; 512];
    log[0] = 0b1;
    log[1..3].copy_from_slice(&313u16.to_le_bytes());
    log[3] = 5;
    log[4] = 10;
    log[5] = 41;
    log[160] = 3;
    assert_eq!(
        nvme_health(&log),
        Some(NvmeHealth {
            critical_warning: 1,
            temperature_c: 40,
            available_spare_pct: 5,
            spare_threshold_pct: 10,
            percentage_used: 41,
            media_errors: 3,
        })
    );
    assert_eq!(nvme_health(&log[..100]), None);
}

#[test]
fn low_space_on_the_system_volume() {
    let mut inputs = healthy();
    inputs.volumes = vec![volume(72, 900)];
    let found = evaluate(&inputs);
    let low = find(&found, FindingKind::LowSpace).unwrap();
    assert_eq!(low.concern, Concern::Attention);
    assert_eq!(low.subject, Subject::Volume(r"C:\".into()));
    assert_eq!(low.reference, Reference::AtLeast(Measure::Percent(10.0)));

    inputs.volumes = vec![volume(40, 900)];
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::LowSpace)
            .unwrap()
            .concern,
        Concern::Critical
    );

    inputs.volumes[0].system = false;
    assert_eq!(find(&evaluate(&inputs), FindingKind::LowSpace), None);
}

#[test]
fn finding_needs_margin_to_resolve() {
    let mut inputs = healthy();
    inputs.volumes = vec![volume(99, 900)];
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::LowSpace),
        None,
        "11% free does not open a finding"
    );

    inputs.open = vec![(FindingKind::LowSpace, Subject::Volume(r"C:\".into()))];
    assert!(
        find(&evaluate(&inputs), FindingKind::LowSpace).is_some(),
        "an open finding stays until 12% free"
    );

    inputs.volumes = vec![volume(117, 900)];
    assert_eq!(find(&evaluate(&inputs), FindingKind::LowSpace), None);
}

#[test]
fn page_file_off_or_commit_near_the_limit() {
    let mut inputs = healthy();
    inputs.page_files = Some(Vec::new());
    let found = evaluate(&inputs);
    let off = find(&found, FindingKind::PageFile).unwrap();
    assert_eq!(off.concern, Concern::Attention);
    assert!(off.evidence.contains(&HealthFact::NoPageFile));
    assert_eq!(off.next_steps[0], NextStep::LetWindowsManagePageFile);

    inputs.commit_peak_pct_7d = Some(94.0);
    let found = evaluate(&inputs);
    let full = find(&found, FindingKind::PageFile).unwrap();
    assert_eq!(full.concern, Concern::Critical);
    assert_eq!(full.value, Measure::Percent(94.0));

    let mut with_file = healthy();
    with_file.events = vec![event(
        2,
        "Microsoft-Windows-Resource-Exhaustion-Detector",
        2004,
        EventMeaning::LowVirtualMemory,
        "",
    )];
    assert_eq!(
        find(&evaluate(&with_file), FindingKind::PageFile)
            .unwrap()
            .concern,
        Concern::Critical
    );

    with_file.page_files = None;
    assert_eq!(
        find(&evaluate(&with_file), FindingKind::PageFile),
        None,
        "a source not read finds nothing"
    );
}

#[test]
fn memory_high_on_three_days_and_its_margin() {
    let mut inputs = healthy();
    inputs.memory_high_days_7d = 3;
    assert!(find(&evaluate(&inputs), FindingKind::MemoryOften).is_some());

    inputs.memory_high_days_7d = 2;
    assert_eq!(find(&evaluate(&inputs), FindingKind::MemoryOften), None);

    inputs.open = vec![(FindingKind::MemoryOften, Subject::Pc)];
    assert!(find(&evaluate(&inputs), FindingKind::MemoryOften).is_some());

    inputs.memory_high_days_7d = 1;
    assert_eq!(find(&evaluate(&inputs), FindingKind::MemoryOften), None);
}

#[test]
fn display_resets_count_the_last_week() {
    let mut inputs = healthy();
    let reset = |days| event(days, "Display", 4101, EventMeaning::DisplayReset, "");
    inputs.events = vec![reset(1), reset(3), reset(9)];

    let found = evaluate(&inputs);
    let resets = find(&found, FindingKind::DisplayResets).unwrap();
    assert_eq!(resets.concern, Concern::Attention);
    assert_eq!(resets.value, Measure::Count(2));
    assert_eq!(resets.evidence.len(), 2);

    inputs.events = (0..5).map(reset).collect();
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::DisplayResets)
            .unwrap()
            .concern,
        Concern::Critical
    );
}

#[test]
fn hardware_and_storage_errors() {
    let whea = |id| {
        event(
            4,
            "Microsoft-Windows-WHEA-Logger",
            id,
            EventMeaning::HardwareError,
            "",
        )
    };
    let mut inputs = healthy();
    inputs.events = vec![whea(19)];
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::HardwareErrors)
            .unwrap()
            .concern,
        Concern::Attention
    );
    inputs.events.push(whea(18));
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::HardwareErrors)
            .unwrap()
            .concern,
        Concern::Critical
    );

    inputs.events = vec![event(10, "storahci", 129, EventMeaning::StorageReset, "")];
    let found = evaluate(&inputs);
    let storage = find(&found, FindingKind::StorageErrors).unwrap();
    assert_eq!(storage.concern, Concern::Attention);
    assert!(storage.next_steps.contains(&NextStep::UpdateStorageDriver));

    inputs
        .events
        .push(event(20, "disk", 7, EventMeaning::DiskBadBlock, ""));
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::StorageErrors)
            .unwrap()
            .concern,
        Concern::Critical
    );

    inputs.events = vec![event(40, "disk", 7, EventMeaning::DiskBadBlock, "")];
    assert_eq!(find(&evaluate(&inputs), FindingKind::StorageErrors), None);
}

#[test]
fn throttling_and_windows_on_hdd() {
    let mut inputs = healthy();
    inputs.events = vec![event(
        2,
        "Microsoft-Windows-Kernel-Processor-Power",
        37,
        EventMeaning::CpuFirmwareLimit,
        "",
    )];
    inputs.system_disk = Some("1 D:".into());

    let found = evaluate(&inputs);

    assert!(find(&found, FindingKind::Throttling).is_some());
    let hdd = find(&found, FindingKind::WindowsOnHdd).unwrap();
    assert_eq!(hdd.value, Measure::Media(Media::Hdd));
    assert_eq!(hdd.reference, Reference::Is(Measure::Media(Media::Ssd)));
}

#[test]
fn two_antivirus_on() {
    let mut inputs = healthy();
    inputs.antivirus = Some(vec![product("First", true), product("Second", false)]);
    assert_eq!(find(&evaluate(&inputs), FindingKind::TwoAntivirus), None);

    inputs.antivirus = Some(vec![product("First", true), product("Second", true)]);
    let found = evaluate(&inputs);
    let two = find(&found, FindingKind::TwoAntivirus).unwrap();
    assert_eq!(two.value, Measure::Count(2));
    assert_eq!(two.next_steps, [NextStep::KeepOneAntivirus]);
}

#[test]
fn slow_boot_names_what_slowed_it_and_spares_security() {
    let boot = |days: i64, boot_ms| BootTime {
        ts_ms: NOW - days * DAY,
        boot_ms,
        main_path_ms: boot_ms / 2,
        post_boot_ms: boot_ms / 2,
    };
    let delay = |kind, security| BootDelay {
        ts_ms: NOW - 2 * DAY,
        kind,
        name: "some.exe".into(),
        total_ms: 9_000,
        degradation_ms: 4_000,
        security,
    };
    let mut inputs = healthy();
    inputs.boot = Some(BootReport {
        boots: vec![boot(20, 50_000), boot(2, 95_000)],
        delays: Vec::new(),
    });
    let found = evaluate(&inputs);
    let slow = find(&found, FindingKind::SlowBoot).unwrap();
    assert_eq!(slow.value, Measure::Seconds(95.0));
    assert!(slow.next_steps.is_empty());

    inputs.boot = Some(BootReport {
        boots: vec![boot(2, 50_000)],
        delays: vec![delay(DelayKind::App, true)],
    });
    let found = evaluate(&inputs);
    assert!(
        find(&found, FindingKind::SlowBoot)
            .unwrap()
            .next_steps
            .is_empty(),
        "a security product is never offered for removal"
    );

    inputs
        .boot
        .as_mut()
        .unwrap()
        .delays
        .push(delay(DelayKind::App, false));
    let found = evaluate(&inputs);
    assert_eq!(
        find(&found, FindingKind::SlowBoot).unwrap().next_steps,
        [NextStep::ReviewStartupApps]
    );

    inputs.boot = Some(BootReport {
        boots: vec![boot(40, 120_000)],
        delays: Vec::new(),
    });
    assert_eq!(find(&evaluate(&inputs), FindingKind::SlowBoot), None);
}

#[test]
fn falls_while_shutting_down_are_not_crashes() {
    let mut inputs = healthy();
    inputs.events = vec![
        power_loss(1, 0, 6),
        power_loss(3, 0, 6),
        power_loss(8, 0, 6),
    ];

    let found = evaluate(&inputs);

    let unfinished = find(&found, FindingKind::ShutdownUnfinished).unwrap();
    assert_eq!(unfinished.value, Measure::Count(3));
    assert_eq!(unfinished.next_steps, [NextStep::WaitForShutdown]);
    assert_eq!(find(&found, FindingKind::DirtyRestarts), None);
}

#[test]
fn dirty_restarts_and_blue_screens() {
    let mut inputs = healthy();
    inputs.events = vec![power_loss(5, 0, 0), power_loss(20, 0, 0)];
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::DirtyRestarts)
            .unwrap()
            .concern,
        Concern::Attention
    );

    inputs.events = vec![power_loss(3, 80, 0)];
    let found = evaluate(&inputs);
    let crash = find(&found, FindingKind::DirtyRestarts).unwrap();
    assert_eq!(crash.concern, Concern::Critical);
    assert_eq!(crash.value, Measure::Count(1));

    inputs.events = vec![power_loss(10, 80, 0)];
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::DirtyRestarts),
        None,
        "one old fall is not a pattern"
    );
}

#[test]
fn failures_rising_against_the_weeks_before() {
    let mut inputs = healthy();
    inputs.events = vec![app_crash(1), app_crash(2), app_crash(3), app_crash(4)];
    inputs.events.extend([app_crash(10), app_crash(30)]);
    let found = evaluate(&inputs);
    let rising = find(&found, FindingKind::FailuresRising).unwrap();
    assert_eq!(rising.value, Measure::Count(4));

    inputs.events.extend((8..20).map(app_crash));
    assert_eq!(
        find(&evaluate(&inputs), FindingKind::FailuresRising),
        None,
        "four a week is not a rise when the weeks before had as many"
    );

    let mut young = healthy();
    young.history_from_ms = NOW - 10 * DAY;
    young.events = vec![app_crash(1), app_crash(2), app_crash(3)];
    assert_eq!(
        find(&evaluate(&young), FindingKind::FailuresRising),
        None,
        "a trend needs five weeks of history"
    );
}

#[test]
fn no_crash_dump_warning() {
    let mut inputs = healthy();
    inputs.events = vec![event(6, "volmgr", 46, EventMeaning::NoCrashDump, "")];

    let found = evaluate(&inputs);

    assert_eq!(
        find(&found, FindingKind::NoCrashDump).unwrap().next_steps,
        [NextStep::LetWindowsManagePageFile]
    );
}

#[test]
fn every_finding_carries_evidence_and_critical_comes_first() {
    let mut inputs = healthy();
    inputs.disks = vec![disk(vec![attr(197, 2)])];
    inputs.volumes = vec![volume(30, 900)];
    inputs.page_files = Some(Vec::new());
    inputs.memory_high_days_7d = 4;
    inputs.system_disk = Some("1 D:".into());
    inputs.antivirus = Some(vec![product("First", true), product("Second", true)]);
    inputs.boot = Some(BootReport {
        boots: vec![BootTime {
            ts_ms: NOW - DAY,
            boot_ms: 100_000,
            main_path_ms: 60_000,
            post_boot_ms: 40_000,
        }],
        delays: Vec::new(),
    });
    inputs.events = vec![
        power_loss(1, 0, 6),
        power_loss(2, 0, 6),
        power_loss(3, 0, 0),
        power_loss(4, 0, 0),
        event(1, "Display", 4101, EventMeaning::DisplayReset, ""),
        event(2, "Display", 4101, EventMeaning::DisplayReset, ""),
        event(
            3,
            "Microsoft-Windows-WHEA-Logger",
            17,
            EventMeaning::HardwareError,
            "",
        ),
        event(4, "stornvme", 129, EventMeaning::StorageReset, ""),
        event(
            5,
            "Microsoft-Windows-Kernel-Processor-Power",
            37,
            EventMeaning::CpuFirmwareLimit,
            "",
        ),
        event(6, "volmgr", 46, EventMeaning::NoCrashDump, ""),
    ];

    let found = evaluate(&inputs);

    assert_eq!(found.len(), 15, "every rule fired");
    for f in &found {
        assert!(!f.evidence.is_empty(), "{:?} has no evidence", f.kind);
    }
    let first_attention = found
        .iter()
        .position(|f| f.concern == Concern::Attention)
        .unwrap();
    assert!(
        found[..first_attention]
            .iter()
            .all(|f| f.concern == Concern::Critical)
    );
    assert!(
        found[first_attention..]
            .iter()
            .all(|f| f.concern == Concern::Attention)
    );
}

#[test]
fn reconcile_opens_updates_and_closes() {
    let mut inputs = healthy();
    inputs.memory_high_days_7d = 3;
    inputs.volumes = vec![volume(72, 900)];
    let fresh = evaluate(&inputs);
    let memory = find(&fresh, FindingKind::MemoryOften).unwrap().clone();
    let space = find(&fresh, FindingKind::LowSpace).unwrap().clone();

    let mut changed = memory.clone();
    changed.value = Measure::Days(5);
    let mut gone = space.clone();
    gone.kind = FindingKind::Throttling;
    let open = vec![
        StoredFinding {
            id: 1,
            opened_ms: NOW - DAY,
            finding: changed,
        },
        StoredFinding {
            id: 2,
            opened_ms: NOW - DAY,
            finding: gone,
        },
    ];

    let changes = reconcile(&open, &fresh);

    assert_eq!(
        changes,
        vec![
            FindingChange::Open(space.clone()),
            FindingChange::Update {
                id: 1,
                finding: memory.clone()
            },
            FindingChange::Close { id: 2 },
        ]
    );
    let settled = vec![StoredFinding {
        id: 1,
        opened_ms: NOW - DAY,
        finding: memory.clone(),
    }];
    assert_eq!(
        reconcile(&settled, std::slice::from_ref(&memory)),
        Vec::new()
    );
}

#[test]
fn boot_events_fold_into_a_report() {
    let mut report = BootReport::default();
    let boot = "<EventData><Data Name='BootTime'>89066</Data>\
                <Data Name='MainPathBootTime'>57066</Data>\
                <Data Name='BootPostBootTime'>32000</Data></EventData>";
    let defender = "<EventData><Data Name='Name'>MsMpEng.exe</Data>\
                    <Data Name='FriendlyName'>Built-in Antimalware Service</Data>\
                    <Data Name='TotalTime'>14309</Data>\
                    <Data Name='DegradationTime'>5809</Data></EventData>";
    let app = "<EventData><Data Name='Name'>tray.exe</Data>\
               <Data Name='FriendlyName'></Data>\
               <Data Name='TotalTime'>9000</Data>\
               <Data Name='DegradationTime'>4000</Data></EventData>";
    let phase = "<EventData><Data Name='Name'>PreShellInit</Data>\
                 <Data Name='TotalTime'>39496</Data>\
                 <Data Name='DegradationTime'>21117</Data></EventData>";

    assert!(report.add(100, 1, boot));
    assert!(report.add(101, 2, defender));
    assert!(report.add(101, 3, app));
    assert!(report.add(108, 4, phase));
    assert!(
        !report.add(200, 5, boot),
        "shutdown events are not boot steps"
    );
    assert!(!report.add(101, 6, "<EventData></EventData>"));
    assert!(!report.add(100, 7, "garbage"));

    assert_eq!(
        report.boots,
        vec![BootTime {
            ts_ms: 1,
            boot_ms: 89_066,
            main_path_ms: 57_066,
            post_boot_ms: 32_000,
        }]
    );
    let kinds: Vec<_> = report
        .delays
        .iter()
        .map(|d| (d.kind, d.name.as_str(), d.security, d.degradation_ms))
        .collect();
    assert_eq!(
        kinds,
        [
            (DelayKind::App, "Built-in Antimalware Service", true, 5_809),
            (DelayKind::App, "tray.exe", false, 4_000),
            (DelayKind::Phase, "PreShellInit", false, 21_117),
        ]
    );
}
