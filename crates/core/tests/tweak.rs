use vestigium_core::SettingStore as _;
use vestigium_core::health::{
    BootDelay, Concern, DelayKind, Finding, FindingKind, HealthFact, Measure, Reference, Subject,
};
use vestigium_core::hwtest::{Outcome, TestData, TestRun, VolumeCheck, VolumeResult};
use vestigium_core::{
    CaptureCauses, Cause, Channel, Culprit, Episode, EpisodeKind, EventMeaning, Hive,
    OccurrenceKind, OccurrenceSummary, SCHEME_BALANCED, SCHEME_POWER_SAVER, Setting, SettingChange,
    Severity, Signal, StartupEntry, StartupList, Status, TWEAKS, TweakId, TweakInputs, Value,
    WinEvent, advise, disabled_approval, helped, undo,
};

const DAY: i64 = 24 * 60 * 60_000;
const NOW: i64 = 1_800_000_000_000;

fn inputs() -> TweakInputs {
    TweakInputs {
        now_ms: NOW,
        evidence_from_ms: NOW - 30 * DAY,
        values: Vec::new(),
        startup: Vec::new(),
        hiberfile_bytes: None,
        system_root: r"C:\".into(),
        system_disk: Some("0 C:".into()),
        monitors: 1,
        battery: false,
        findings: Vec::new(),
        occurrences: Vec::new(),
        episodes: Vec::new(),
        captures: Vec::new(),
        events: Vec::new(),
        tests: Vec::new(),
    }
}

fn with(mut inputs: TweakInputs, tweak: TweakId, values: &[Value]) -> TweakInputs {
    for (setting, value) in tweak.settings().into_iter().zip(values) {
        inputs.values.push((setting, value.clone()));
    }
    inputs
}

fn finding(kind: FindingKind, subject: Subject, evidence: Vec<HealthFact>) -> Finding {
    Finding {
        kind,
        subject,
        concern: Concern::Attention,
        value: Measure::Count(1),
        reference: Reference::AtMost(Measure::Count(0)),
        evidence,
        next_steps: Vec::new(),
    }
}

fn shutdown_fall(days_ago: i64) -> WinEvent {
    WinEvent {
        ts_ms: NOW - days_ago * DAY,
        channel: Channel::System,
        provider: "Microsoft-Windows-Kernel-Power".into(),
        id: 41,
        record_id: days_ago as u64,
        level: 1,
        meaning: EventMeaning::PowerLoss,
        data_xml: "<EventData><Data Name='BugcheckCode'>0</Data>\
                   <Data Name='SleepInProgress'>6</Data></EventData>"
            .into(),
    }
}

fn occurrence(at: i64, kind: OccurrenceKind, cause: Cause) -> OccurrenceSummary {
    OccurrenceSummary {
        id: at,
        kind,
        start_ms: at,
        end_ms: at + 1000,
        severity: Severity::Medium,
        cause,
    }
}

fn episode(at: i64, kind: EpisodeKind, culprit: &str) -> Episode {
    Episode {
        kind,
        start_ms: at,
        end_ms: at + 60_000,
        peak: 100.0,
        culprit: Some(Culprit {
            pid: 4321,
            name: culprit.into(),
            value: 90.0,
        }),
        truncated: false,
    }
}

fn advice_for(inputs: &TweakInputs, tweak: TweakId) -> vestigium_core::Advice {
    advise(inputs)
        .into_iter()
        .find(|a| a.tweak == tweak)
        .expect("advised")
}

#[test]
fn catalog_never_touches_security() {
    for tweak in TWEAKS {
        for setting in tweak.settings() {
            assert!(!setting.is_forbidden(), "{tweak:?} touches {setting:?}");
        }
    }
    let defender = Setting::Service {
        name: "WinDefend".into(),
    };
    let uac = Setting::Dword {
        hive: Hive::LocalMachine,
        key: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System".into(),
        name: "EnableLUA".into(),
    };
    let page_file = Setting::Dword {
        hive: Hive::LocalMachine,
        key: r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management".into(),
        name: "PagingFiles".into(),
    };
    assert!(defender.is_forbidden() && uac.is_forbidden() && page_file.is_forbidden());
}

#[test]
fn every_tweak_has_an_undo_in_its_domain() {
    let all = inputs();
    let all = with(all, TweakId::FastStartup, &[Value::Dword(1)]);
    let all = with(all, TweakId::Hibernation, &[Value::On(true)]);
    let all = with(
        all,
        TweakId::PowerPlan,
        &[Value::Scheme(SCHEME_POWER_SAVER)],
    );
    let all = with(all, TweakId::PcieAspm, &[Value::Index(2)]);
    let all = with(all, TweakId::UsbSuspend, &[Value::Index(1)]);
    let service = Value::Service {
        start: 2,
        delayed: true,
        running: true,
    };
    let all = with(all, TweakId::SysMain, std::slice::from_ref(&service));
    let all = with(all, TweakId::WindowsSearch, &[service]);
    let all = with(
        all,
        TweakId::GameDvr,
        &[Value::Dword(1), Value::Absent, Value::Dword(1)],
    );
    let all = with(all, TweakId::GpuScheduling, &[Value::Dword(2)]);
    let all = with(all, TweakId::Mpo, &[Value::Absent]);
    let all = with(
        all,
        TweakId::AppDumps,
        &[Value::Absent, Value::Absent, Value::Absent],
    );
    let all = with(all, TweakId::CrashDump, &[Value::Dword(0)]);
    let advice = advise(&all);
    for tweak in TWEAKS.iter().filter(|t| **t != TweakId::StartupApp) {
        let a = advice.iter().find(|a| a.tweak == *tweak).expect("advised");
        assert_eq!(a.status, Status::Ready, "{tweak:?}");
        assert!(!a.changes.is_empty(), "{tweak:?}");
        for change in &a.changes {
            assert!(
                change.setting.fits(&change.to),
                "{tweak:?} writes {change:?}"
            );
            assert!(
                change.setting.fits(&change.from),
                "{tweak:?} cannot restore {change:?}"
            );
        }
        let back = undo(&a.changes);
        assert_eq!(back.len(), a.changes.len());
        assert_eq!(undo(&back), a.changes);
    }
}

#[test]
fn pcie_aspm_needs_a_storage_or_network_module() {
    let mut stalls = with(inputs(), TweakId::PcieAspm, &[Value::Index(2)]);
    stalls.occurrences = vec![
        occurrence(
            NOW - DAY,
            OccurrenceKind::Stutter,
            Cause::Driver { core: 2 },
        ),
        occurrence(
            NOW - 3 * DAY,
            OccurrenceKind::Stutter,
            Cause::Driver { core: 2 },
        ),
    ];
    let advice = advice_for(&stalls, TweakId::PcieAspm);
    assert_eq!(advice.status, Status::Ready);
    assert!(!advice.recommended(), "driver time alone names no module");

    stalls.captures = vec![CaptureCauses {
        start_ms: NOW - 2 * DAY,
        causes: vec![Cause::DriverModule {
            module: "dxgkrnl.sys".into(),
            intermediary: false,
        }],
    }];
    assert!(!advice_for(&stalls, TweakId::PcieAspm).recommended());

    stalls.captures[0].causes = vec![Cause::DriverModule {
        module: "storahci.sys".into(),
        intermediary: false,
    }];
    let advice = advice_for(&stalls, TweakId::PcieAspm);
    assert!(advice.recommended());
    assert!(
        advice
            .signals
            .iter()
            .any(|s| matches!(s, Signal::DpcIsrStalls { count: 2 }))
    );
}

#[test]
fn already_set_is_not_offered() {
    let set = with(inputs(), TweakId::FastStartup, &[Value::Dword(0)]);
    let set = with(set, TweakId::PowerPlan, &[Value::Scheme(SCHEME_BALANCED)]);
    let set = with(set, TweakId::CrashDump, &[Value::Dword(7)]);
    let set = with(
        set,
        TweakId::AppDumps,
        &[Value::Present, Value::Absent, Value::Absent],
    );
    for tweak in [
        TweakId::FastStartup,
        TweakId::PowerPlan,
        TweakId::CrashDump,
        TweakId::AppDumps,
    ] {
        let a = advice_for(&set, tweak);
        assert_eq!(a.status, Status::AlreadySet, "{tweak:?}");
        assert!(!a.recommended());
    }
    assert_eq!(advice_for(&set, TweakId::Mpo).status, Status::Unavailable);
    let plan = advice_for(&set, TweakId::PowerPlan);
    assert_eq!(plan.current[0].value, Value::Scheme(SCHEME_BALANCED));
    assert!(advice_for(&set, TweakId::Mpo).current.is_empty());
}

#[test]
fn fast_startup_needs_an_unfinished_shutdown() {
    let on = with(inputs(), TweakId::FastStartup, &[Value::Dword(1)]);
    assert!(!advice_for(&on, TweakId::FastStartup).recommended());

    let mut falls = on.clone();
    falls.events = vec![shutdown_fall(2), shutdown_fall(9)];
    falls.findings = vec![finding(
        FindingKind::ShutdownUnfinished,
        Subject::Pc,
        Vec::new(),
    )];
    let a = advice_for(&falls, TweakId::FastStartup);
    assert!(a.recommended());
    assert!(
        a.signals
            .contains(&Signal::UnfinishedShutdowns { count: 2 })
    );
    assert_eq!(
        a.changes,
        vec![SettingChange {
            setting: TweakId::FastStartup.settings()[0].clone(),
            from: Value::Dword(1),
            to: Value::Dword(0),
        }]
    );
}

#[test]
fn hibernation_needs_low_space_on_the_system_drive() {
    let mut on = with(inputs(), TweakId::Hibernation, &[Value::On(true)]);
    on.hiberfile_bytes = Some(12 << 30);
    on.findings = vec![finding(
        FindingKind::LowSpace,
        Subject::Volume(r"D:\".into()),
        Vec::new(),
    )];
    assert!(!advice_for(&on, TweakId::Hibernation).recommended());
    on.findings = vec![finding(
        FindingKind::LowSpace,
        Subject::Volume(r"c:\".into()),
        Vec::new(),
    )];
    assert!(advice_for(&on, TweakId::Hibernation).recommended());
}

#[test]
fn power_saver_only_on_a_pc_without_battery() {
    let mut saver = with(
        inputs(),
        TweakId::PowerPlan,
        &[Value::Scheme(SCHEME_POWER_SAVER)],
    );
    saver.occurrences = (0..3)
        .map(|i| occurrence(NOW - i * DAY, OccurrenceKind::Stutter, Cause::Unclear))
        .collect();
    let a = advice_for(&saver, TweakId::PowerPlan);
    assert!(a.recommended());
    assert_eq!(a.changes[0].to, Value::Scheme(SCHEME_BALANCED));
    saver.battery = true;
    assert!(!advice_for(&saver, TweakId::PowerPlan).recommended());
}

#[test]
fn wsearch_needs_the_indexer_as_culprit() {
    let on = Value::Service {
        start: 2,
        delayed: true,
        running: true,
    };
    let mut busy = with(inputs(), TweakId::WindowsSearch, &[on]);
    let disk = EpisodeKind::DiskBusy {
        disk: "0 C:".into(),
    };
    busy.episodes = vec![
        episode(NOW - DAY, disk.clone(), "Browser.exe"),
        episode(NOW - 2 * DAY, disk.clone(), "Browser.exe"),
    ];
    assert!(!advice_for(&busy, TweakId::WindowsSearch).recommended());
    busy.episodes = vec![
        episode(NOW - DAY, disk.clone(), "SearchIndexer.exe"),
        episode(
            NOW - 2 * DAY,
            EpisodeKind::CpuSaturated,
            "SearchProtocolHost.exe",
        ),
        episode(NOW - 20 * DAY, disk, "SearchIndexer.exe"),
    ];
    let a = advice_for(&busy, TweakId::WindowsSearch);
    assert!(a.recommended());
    assert!(a.signals.contains(&Signal::DiskBusy {
        count: 1,
        culprit: Some("SearchIndexer.exe".into()),
    }));
}

fn volume_test_at(at: i64) -> TestRun {
    TestRun {
        subject: Subject::Volume(r"C:\".into()),
        started_ms: at - 30_000,
        ended_ms: at + 30_000,
        outcome: Outcome::Clean,
        data: TestData::VolumeCheck(VolumeCheck {
            volume: r"C:\".into(),
            exit_code: Some(0),
            result: VolumeResult::NoProblems,
            output: String::new(),
            dirty_before: None,
            dirty_after: None,
        }),
    }
}

#[test]
fn system_disk_episodes_caused_by_a_test_are_not_disk_evidence() {
    let on = Value::Service {
        start: 2,
        delayed: true,
        running: true,
    };
    let mut busy = with(inputs(), TweakId::SysMain, &[on]);
    busy.findings = vec![finding(
        FindingKind::WindowsOnHdd,
        Subject::Disk("0 C:".into()),
        Vec::new(),
    )];
    let disk = EpisodeKind::DiskBusy {
        disk: "0 C:".into(),
    };
    busy.episodes = vec![
        episode(NOW - DAY, disk.clone(), "Updater.exe"),
        episode(NOW - 2 * DAY, disk.clone(), "Updater.exe"),
    ];
    let signals = |inputs: &TweakInputs| advice_for(inputs, TweakId::SysMain).signals;
    assert!(
        signals(&busy)
            .iter()
            .any(|s| matches!(s, Signal::DiskBusy { count: 2, .. })),
        "two ordinary episodes are evidence"
    );

    busy.tests = vec![volume_test_at(NOW - DAY), volume_test_at(NOW - 2 * DAY)];
    assert!(
        !signals(&busy)
            .iter()
            .any(|s| matches!(s, Signal::DiskBusy { .. })),
        "the load of a test is not"
    );
    assert!(!advice_for(&busy, TweakId::SysMain).recommended());

    // The same episodes in the before window do not count toward "helped" either.
    busy.episodes = vec![episode(NOW - 8 * DAY, disk, "chkdsk.exe")];
    busy.tests = vec![volume_test_at(NOW - 8 * DAY)];
    let result = helped(TweakId::SysMain, NOW - 5 * DAY, &busy).unwrap();
    assert_eq!((result.before, result.after), (0, 0));
}

#[test]
fn gpu_scheduling_flips_the_current_state() {
    let on = with(inputs(), TweakId::GpuScheduling, &[Value::Dword(2)]);
    assert_eq!(
        advice_for(&on, TweakId::GpuScheduling).changes[0].to,
        Value::Dword(1)
    );
    let off = with(inputs(), TweakId::GpuScheduling, &[Value::Dword(1)]);
    assert_eq!(
        advice_for(&off, TweakId::GpuScheduling).changes[0].to,
        Value::Dword(2)
    );
    let unsupported = with(inputs(), TweakId::GpuScheduling, &[Value::Absent]);
    assert_eq!(
        advice_for(&unsupported, TweakId::GpuScheduling).status,
        Status::Unavailable
    );
}

#[test]
fn startup_app_is_never_a_security_product() {
    let mut boot = inputs();
    let delay = |name: &str, security: bool| {
        HealthFact::Delay(BootDelay {
            ts_ms: NOW - DAY,
            kind: DelayKind::App,
            name: name.into(),
            total_ms: 9000,
            degradation_ms: 6000,
            security,
        })
    };
    boot.findings = vec![finding(
        FindingKind::SlowBoot,
        Subject::Pc,
        vec![
            delay("Chat Client", false),
            delay("Endpoint Security", true),
        ],
    )];
    boot.startup = vec![
        StartupEntry {
            hive: Hive::CurrentUser,
            list: StartupList::Run,
            name: "ChatClient".into(),
            exe: "Chat.exe".into(),
        },
        StartupEntry {
            hive: Hive::LocalMachine,
            list: StartupList::Run,
            name: "SecurityTray".into(),
            exe: "EndpointSecurityTray.exe".into(),
        },
    ];
    let advice = advise(&boot);
    let chat = advice
        .iter()
        .find(|a| a.subject.as_deref() == Some("ChatClient"))
        .expect("chat");
    assert!(chat.recommended());
    assert_eq!(chat.applier, vestigium_core::Applier::User);
    assert_eq!(chat.changes[0].from, Value::Absent);
    let security = advice
        .iter()
        .find(|a| a.subject.as_deref() == Some("SecurityTray"))
        .expect("security");
    assert_eq!(security.status, Status::Protected);
    assert!(security.changes.is_empty() && security.signals.is_empty());
}

fn startup_status(name: &str, exe: &str) -> Status {
    let mut pc = inputs();
    pc.startup = vec![StartupEntry {
        hive: Hive::CurrentUser,
        list: StartupList::Run,
        name: name.into(),
        exe: exe.into(),
    }];
    advise(&pc)
        .into_iter()
        .find(|a| a.subject.as_deref() == Some(name))
        .expect("the entry")
        .status
}

#[test]
fn security_products_are_protected_by_vendor_name() {
    for (name, exe) in [
        ("Malwarebytes Tray", "mbamtray.exe"),
        (
            "MBAMTray",
            r"C:\Program Files\Malwarebytes\Anti-Malware\mbamtray.exe",
        ),
        ("Acme Anti-Malware Helper", "acme.exe"),
        ("Acme Anti-Virus Helper", "acme.exe"),
        ("Kaspersky Tray", "avp.exe"),
        ("McAfee Agent", r"C:\Program Files\McAfee\Agent\masvc.exe"),
        (
            "Avast Tray",
            r"C:\Program Files\Avast Software\Avast\AvastUI.exe",
        ),
        ("Norton Tray", "nortontray.exe"),
        ("Sophos UI", "sophosui.exe"),
        ("ESET Tray", "egui.exe"),
    ] {
        assert_eq!(startup_status(name, exe), Status::Protected, "{name}");
    }
}

#[test]
fn ordinary_startup_entries_stay_ready() {
    for (name, exe) in [
        (
            "Spotify",
            r"C:\Users\Someone\AppData\Roaming\Spotify\Spotify.exe",
        ),
        (
            "OneDrive",
            r"C:\Program Files\Microsoft OneDrive\OneDrive.exe",
        ),
        ("Steam Client", "steam.exe"),
        ("Reset Helper", "reset.exe"),
    ] {
        assert_eq!(startup_status(name, exe), Status::Ready, "{name}");
    }
}

#[test]
fn a_third_party_antivirus_startup_entry_is_protected() {
    for exe in ["AvastUI.exe", "avgui.exe", "egui.exe", "avpui.exe"] {
        let mut pc = inputs();
        pc.startup = vec![StartupEntry {
            hive: Hive::CurrentUser,
            list: StartupList::Run,
            name: exe.into(),
            exe: exe.into(),
        }];
        let advice = advise(&pc);
        let entry = advice
            .iter()
            .find(|a| a.subject.as_deref() == Some(exe))
            .expect("the entry");
        assert_eq!(entry.status, Status::Protected, "{exe}");
        assert!(entry.changes.is_empty());

        let approval = pc.startup[0].approval();
        assert!(approval.is_forbidden(), "{exe} is never disabled");
        let mut plan = item(
            TweakId::StartupApp,
            vec![(approval, Value::Bytes(disabled_approval(NOW)))],
            None,
        );
        plan.subject = Some(exe.into());
        assert!(
            vestigium_core::check_item(&plan, vestigium_core::Applier::User).is_err(),
            "{exe} is refused"
        );
    }
}

#[test]
fn a_word_inside_another_word_is_not_a_security_product() {
    for name in ["Reset Helper", "ResetTool.exe", "Nuclear.exe"] {
        let mut pc = inputs();
        pc.startup = vec![StartupEntry {
            hive: Hive::CurrentUser,
            list: StartupList::Run,
            name: name.into(),
            exe: name.into(),
        }];
        let advice = advise(&pc);
        let entry = advice
            .iter()
            .find(|a| a.subject.as_deref() == Some(name))
            .expect("the entry");
        assert_eq!(entry.status, Status::Ready, "{name}");
    }
}

#[test]
fn a_startup_delay_is_not_matched_by_four_letters() {
    let mut boot = inputs();
    boot.findings = vec![finding(
        FindingKind::SlowBoot,
        Subject::Pc,
        vec![HealthFact::Delay(BootDelay {
            ts_ms: NOW - DAY,
            kind: DelayKind::App,
            name: "Dropbox Sync Helper".into(),
            total_ms: 9000,
            degradation_ms: 6000,
            security: false,
        })],
    )];
    boot.startup = vec![StartupEntry {
        hive: Hive::CurrentUser,
        list: StartupList::Run,
        name: "Sync".into(),
        exe: "Sync.exe".into(),
    }];
    let advice = advise(&boot);
    let sync = advice
        .iter()
        .find(|a| a.subject.as_deref() == Some("Sync"))
        .expect("sync");
    assert!(
        sync.signals
            .iter()
            .all(|s| !matches!(s, Signal::SlowStartup { .. })),
        "no startup delay is blamed on Sync"
    );
}

#[test]
fn startup_disable_writes_03_and_the_time() {
    let bytes = disabled_approval(0);
    assert_eq!(bytes.len(), 12);
    assert_eq!(&bytes[..4], &[3, 0, 0, 0]);
    let filetime = u64::from_le_bytes(bytes[4..].try_into().unwrap());
    assert_eq!(filetime, 116_444_736_000_000_000);
}

#[test]
fn helped_counts_equal_windows() {
    let mut after = with(inputs(), TweakId::GameDvr, &[]);
    let applied = NOW - 3 * DAY;
    after.occurrences = [5, 4, 2, -1]
        .into_iter()
        .map(|d| {
            occurrence(
                applied - d * DAY + 1,
                OccurrenceKind::Stutter,
                Cause::Unclear,
            )
        })
        .collect();
    let h = helped(TweakId::GameDvr, applied, &after).expect("counted");
    assert_eq!((h.days, h.before, h.after), (3, 1, 1));
    assert!(helped(TweakId::GameDvr, NOW - DAY / 2, &after).is_none());
    assert!(helped(TweakId::CrashDump, applied, &after).is_none());
}

#[test]
fn helped_is_withheld_when_the_before_window_leaves_the_evidence() {
    let mut after = with(inputs(), TweakId::GameDvr, &[]);
    let stutter = |at| occurrence(at, OccurrenceKind::Stutter, Cause::Unclear);
    after.occurrences = vec![stutter(NOW - 29 * DAY), stutter(NOW - 26 * DAY)];

    let inside = NOW - 20 * DAY;
    assert!(helped(TweakId::GameDvr, inside, &after).is_some());
    let edge = NOW - 23 * DAY;
    assert!(
        helped(TweakId::GameDvr, edge, &after).is_some(),
        "a before window that starts exactly at the evidence is complete"
    );
    let truncated = NOW - 25 * DAY;
    assert!(helped(TweakId::GameDvr, truncated, &after).is_none());
    let beyond = NOW - 40 * DAY;
    assert!(helped(TweakId::GameDvr, beyond, &after).is_none());
}

struct FakeStore {
    values: Vec<(Setting, Value)>,
    fails: Option<Setting>,
    writes: usize,
}

impl vestigium_core::SettingStore for FakeStore {
    fn read(&mut self, setting: &Setting) -> Option<Value> {
        self.values
            .iter()
            .find(|(s, _)| s == setting)
            .map(|(_, v)| v.clone())
    }

    fn write(&mut self, setting: &Setting, value: &Value) -> Result<(), u32> {
        if self.fails.as_ref() == Some(setting) {
            return Err(5);
        }
        self.writes += 1;
        match self.values.iter_mut().find(|(s, _)| s == setting) {
            Some((_, v)) => *v = value.clone(),
            None => self.values.push((setting.clone(), value.clone())),
        }
        Ok(())
    }
}

fn item(
    tweak: TweakId,
    changes: Vec<(Setting, Value)>,
    undo_of: Option<i64>,
) -> vestigium_core::PlanItem {
    vestigium_core::PlanItem {
        tweak,
        subject: None,
        undo_of,
        changes: changes
            .into_iter()
            .map(|(setting, to)| SettingChange {
                setting,
                from: Value::Absent,
                to,
            })
            .collect(),
    }
}

#[test]
fn refuses_a_key_outside_the_catalog() {
    let elsewhere = Setting::Dword {
        hive: Hive::LocalMachine,
        key: r"SOFTWARE\Elsewhere".into(),
        name: "OverlayTestMode".into(),
    };
    let plan = item(TweakId::Mpo, vec![(elsewhere, Value::Dword(5))], None);
    assert!(vestigium_core::check_item(&plan, vestigium_core::Applier::Elevated).is_err());
    let mpo = TweakId::Mpo.settings()[0].clone();
    let plan = item(TweakId::Mpo, vec![(mpo, Value::Dword(5))], None);
    assert!(vestigium_core::check_item(&plan, vestigium_core::Applier::Elevated).is_ok());
}

#[test]
fn refuses_a_security_service() {
    let defender = Setting::Service {
        name: "WinDefend".into(),
    };
    let stopped = Value::Service {
        start: 4,
        delayed: false,
        running: false,
    };
    let plan = item(TweakId::SysMain, vec![(defender, stopped)], None);
    assert!(vestigium_core::check_item(&plan, vestigium_core::Applier::Elevated).is_err());
    let mut tray = item(
        TweakId::StartupApp,
        vec![(
            Setting::StartupApproval {
                hive: Hive::LocalMachine,
                list: StartupList::Run,
                name: "SecurityHealth".into(),
            },
            Value::Bytes(disabled_approval(NOW)),
        )],
        None,
    );
    tray.subject = Some("SecurityHealth".into());
    assert!(vestigium_core::check_item(&tray, vestigium_core::Applier::Elevated).is_err());
}

#[test]
fn a_user_setting_is_never_written_elevated() {
    let dvr = TweakId::GameDvr.settings()[0].clone();
    let plan = item(TweakId::GameDvr, vec![(dvr, Value::Dword(0))], None);
    assert!(vestigium_core::check_item(&plan, vestigium_core::Applier::Elevated).is_err());
    assert!(vestigium_core::check_item(&plan, vestigium_core::Applier::User).is_ok());
}

#[test]
fn undo_value_must_fit_the_setting() {
    let mpo = TweakId::Mpo.settings()[0].clone();
    let odd = item(
        TweakId::Mpo,
        vec![(mpo.clone(), Value::Bytes(vec![1; 12]))],
        Some(4),
    );
    assert!(vestigium_core::check_item(&odd, vestigium_core::Applier::Elevated).is_err());
    let back = item(TweakId::Mpo, vec![(mpo, Value::Dword(3))], Some(4));
    assert!(vestigium_core::check_item(&back, vestigium_core::Applier::Elevated).is_ok());
}

#[test]
fn carry_out_records_what_was_there_and_stops_at_the_first_failure() {
    let settings = TweakId::AppDumps.settings();
    let mut store = FakeStore {
        values: settings
            .iter()
            .map(|s| (s.clone(), Value::Absent))
            .collect(),
        fails: Some(settings[1].clone()),
        writes: 0,
    };
    let plan = item(
        TweakId::AppDumps,
        vec![
            (settings[0].clone(), Value::Present),
            (settings[1].clone(), Value::Dword(1)),
            (settings[2].clone(), Value::Dword(10)),
        ],
        None,
    );
    let done = vestigium_core::carry_out(&plan, &mut store);
    assert_eq!(done.results.len(), 2);
    assert_eq!(done.results[0].error, None);
    assert_eq!(done.results[0].change.from, Value::Absent);
    assert_eq!(done.results[1].error, Some(5));
    assert_eq!(store.writes, 1);
    assert!(!done.succeeded());
}

#[test]
fn carry_out_refuses_a_setting_changed_since() {
    let crash = TweakId::CrashDump.settings()[0].clone();
    let mut store = FakeStore {
        values: vec![(crash.clone(), Value::Dword(3))],
        fails: None,
        writes: 0,
    };
    let plan = item(TweakId::CrashDump, vec![(crash, Value::Dword(7))], None);
    let done = vestigium_core::carry_out(&plan, &mut store);
    assert_eq!(
        done.results[0].error,
        Some(vestigium_core::ERROR_CHANGED_SINCE)
    );
    assert_eq!(store.writes, 0);
}

fn aspm(scheme: Option<vestigium_core::Guid>) -> Setting {
    Setting::PowerAc {
        subgroup: vestigium_core::SUB_PCIEXPRESS,
        setting: vestigium_core::PCIE_ASPM,
        scheme,
    }
}

fn two_schemes() -> FakeStore {
    FakeStore {
        values: vec![
            (Setting::ActiveScheme, Value::Scheme(SCHEME_POWER_SAVER)),
            (aspm(Some(SCHEME_POWER_SAVER)), Value::Index(2)),
            (aspm(Some(SCHEME_BALANCED)), Value::Index(1)),
        ],
        fails: None,
        writes: 0,
    }
}

fn set(store: &mut FakeStore, setting: Setting, value: Value) {
    store.values.retain(|(s, _)| *s != setting);
    store.values.push((setting, value));
}

fn written_changes(done: &vestigium_core::TweakChange) -> Vec<SettingChange> {
    done.results.iter().map(|r| r.change.clone()).collect()
}

#[test]
fn an_undo_goes_to_the_scheme_that_was_changed() {
    let mut store = two_schemes();
    let apply = item(TweakId::PcieAspm, vec![(aspm(None), Value::Index(0))], None);
    let done = vestigium_core::carry_out(&apply, &mut store);
    assert_eq!(done.results[0].error, None);
    assert_eq!(
        done.results[0].change.setting,
        aspm(Some(SCHEME_POWER_SAVER))
    );
    assert_eq!(done.results[0].change.from, Value::Index(2));

    set(
        &mut store,
        Setting::ActiveScheme,
        Value::Scheme(SCHEME_BALANCED),
    );
    let back = item(TweakId::PcieAspm, Vec::new(), Some(1));
    let back = vestigium_core::PlanItem {
        changes: undo(&written_changes(&done)),
        ..back
    };
    let undone = vestigium_core::carry_out(&back, &mut store);

    assert_eq!(undone.results[0].error, None);
    assert_eq!(
        store.read(&aspm(Some(SCHEME_POWER_SAVER))),
        Some(Value::Index(2))
    );
    assert_eq!(
        store.read(&aspm(Some(SCHEME_BALANCED))),
        Some(Value::Index(1))
    );
}

#[test]
fn an_undo_is_refused_when_its_scheme_is_gone() {
    let mut store = two_schemes();
    let apply = item(TweakId::PcieAspm, vec![(aspm(None), Value::Index(0))], None);
    let done = vestigium_core::carry_out(&apply, &mut store);
    store
        .values
        .retain(|(s, _)| *s != aspm(Some(SCHEME_POWER_SAVER)));
    set(
        &mut store,
        Setting::ActiveScheme,
        Value::Scheme(SCHEME_BALANCED),
    );
    let writes = store.writes;
    let back = vestigium_core::PlanItem {
        changes: undo(&written_changes(&done)),
        ..item(TweakId::PcieAspm, Vec::new(), Some(1))
    };

    let undone = vestigium_core::carry_out(&back, &mut store);

    assert_eq!(
        undone.results[0].error,
        Some(vestigium_core::ERROR_UNREADABLE)
    );
    assert_eq!(store.writes, writes);
    assert_eq!(
        store.read(&aspm(Some(SCHEME_BALANCED))),
        Some(Value::Index(1))
    );
}

#[test]
fn a_power_change_without_a_scheme_is_still_in_the_catalog() {
    for scheme in [None, Some(SCHEME_BALANCED)] {
        let plan = item(
            TweakId::PcieAspm,
            vec![(aspm(scheme), Value::Index(0))],
            None,
        );
        assert!(vestigium_core::check_item(&plan, vestigium_core::Applier::User).is_ok());
    }
    let other = Setting::PowerAc {
        subgroup: vestigium_core::SUB_USB,
        setting: vestigium_core::PCIE_ASPM,
        scheme: None,
    };
    let plan = item(TweakId::PcieAspm, vec![(other, Value::Index(0))], None);
    assert!(vestigium_core::check_item(&plan, vestigium_core::Applier::User).is_err());
}

#[test]
fn a_write_that_may_have_landed_stays_in_the_undo() {
    let service = Setting::Service {
        name: "WSearch".into(),
    };
    let on = Value::Service {
        start: 2,
        delayed: true,
        running: true,
    };
    let off = Value::Service {
        start: 4,
        delayed: false,
        running: false,
    };
    let result = |error| vestigium_core::SettingResult {
        change: SettingChange {
            setting: service.clone(),
            from: on.clone(),
            to: off.clone(),
        },
        error,
    };
    let change = |error| vestigium_core::TweakChange {
        tweak: TweakId::WindowsSearch,
        subject: None,
        undo_of: None,
        results: vec![result(error)],
        restore_point: None,
    };
    assert_eq!(change(None).written().len(), 1);
    assert_eq!(
        change(Some(vestigium_core::ERROR_MAY_HAVE_CHANGED))
            .written()
            .len(),
        1
    );
    assert!(change(Some(5)).written().is_empty());
}

fn undo_item(setting: Setting, from: Value, to: Value) -> vestigium_core::PlanItem {
    vestigium_core::PlanItem {
        tweak: TweakId::CrashDump,
        subject: None,
        undo_of: Some(1),
        changes: vec![SettingChange { setting, from, to }],
    }
}

#[test]
fn a_forged_undo_cannot_write_an_absurd_value() {
    let count = TweakId::AppDumps.settings()[2].clone();
    let forged = vestigium_core::PlanItem {
        tweak: TweakId::AppDumps,
        subject: None,
        undo_of: Some(1),
        changes: vec![SettingChange {
            setting: count.clone(),
            from: Value::Dword(10),
            to: Value::Dword(u32::MAX),
        }],
    };
    assert!(vestigium_core::check_item(&forged, vestigium_core::Applier::Elevated).is_err());
    let honest = vestigium_core::PlanItem {
        changes: vec![SettingChange {
            setting: count,
            from: Value::Dword(10),
            to: Value::Dword(25),
        }],
        ..forged
    };
    assert!(vestigium_core::check_item(&honest, vestigium_core::Applier::Elevated).is_ok());
}

#[test]
fn dword_values_stay_inside_what_the_setting_takes() {
    let dword = |tweak: TweakId, at: usize| tweak.settings()[at].clone();
    let accepts = |setting: &Setting, value: u32| setting.fits(&Value::Dword(value));
    assert!(accepts(&dword(TweakId::CrashDump, 0), 7));
    assert!(!accepts(&dword(TweakId::CrashDump, 0), 8));
    assert!(accepts(&dword(TweakId::AppDumps, 1), 2));
    assert!(!accepts(&dword(TweakId::AppDumps, 1), 3));
    assert!(accepts(&dword(TweakId::GpuScheduling, 0), 2));
    assert!(!accepts(&dword(TweakId::GpuScheduling, 0), 3));
    assert!(accepts(&dword(TweakId::FastStartup, 0), 1));
    assert!(!accepts(&dword(TweakId::FastStartup, 0), 2));
    assert!(accepts(&dword(TweakId::GameDvr, 0), 1));
    assert!(!accepts(&dword(TweakId::GameDvr, 2), 2));
    assert!(!accepts(&dword(TweakId::Mpo, 0), u32::MAX));
    assert!(dword(TweakId::AppDumps, 2).fits(&Value::Absent));
}

#[test]
fn an_undo_runs_only_over_what_vestigium_wrote() {
    let crash = TweakId::CrashDump.settings()[0].clone();
    let mut store = FakeStore {
        values: vec![(crash.clone(), Value::Dword(1))],
        fails: None,
        writes: 0,
    };
    let back = undo_item(crash.clone(), Value::Dword(7), Value::Dword(0));

    let refused = vestigium_core::carry_out(&back, &mut store);

    assert_eq!(
        refused.results[0].error,
        Some(vestigium_core::ERROR_CHANGED_SINCE)
    );
    assert_eq!(store.writes, 0);
    assert_eq!(store.read(&crash), Some(Value::Dword(1)));

    set(&mut store, crash.clone(), Value::Dword(7));
    let done = vestigium_core::carry_out(&back, &mut store);
    assert_eq!(done.results[0].error, None);
    assert_eq!(store.read(&crash), Some(Value::Dword(0)));

    let again = vestigium_core::carry_out(&back, &mut store);
    assert_eq!(again.results[0].error, None, "already undone counts");
    assert_eq!(store.writes, 1);
}

#[test]
fn a_setting_changed_outside_stops_the_undo_before_it_is_planned() {
    let crash = TweakId::CrashDump.settings()[0].clone();
    let written = vec![SettingChange {
        setting: crash.clone(),
        from: Value::Dword(0),
        to: Value::Dword(7),
    }];
    let mut store = FakeStore {
        values: vec![(crash.clone(), Value::Dword(7))],
        fails: None,
        writes: 0,
    };
    assert!(!vestigium_core::changed_outside(&written, &mut store));
    set(&mut store, crash.clone(), Value::Dword(2));
    assert!(vestigium_core::changed_outside(&written, &mut store));
    set(&mut store, crash, Value::Dword(0));
    assert!(
        !vestigium_core::changed_outside(&written, &mut store),
        "already back"
    );
    assert_eq!(store.writes, 0);
}

struct Ignoring(Value);

impl vestigium_core::SettingStore for Ignoring {
    fn read(&mut self, _: &Setting) -> Option<Value> {
        Some(self.0.clone())
    }

    fn write(&mut self, _: &Setting, _: &Value) -> Result<(), u32> {
        Ok(())
    }
}

#[test]
fn a_write_that_does_not_read_back_may_have_changed() {
    let crash = TweakId::CrashDump.settings()[0].clone();
    let plan = item(TweakId::CrashDump, vec![(crash, Value::Dword(7))], None);
    let done = vestigium_core::carry_out(&plan, &mut Ignoring(Value::Dword(0)));
    assert_eq!(
        done.results[0].error,
        Some(vestigium_core::ERROR_MAY_HAVE_CHANGED)
    );
    assert_eq!(done.results[0].change.from, Value::Dword(0));
    assert_eq!(done.results[0].change.to, Value::Dword(7));
}

#[test]
fn a_service_still_starting_reads_back_as_written() {
    let service = Setting::Service {
        name: "SysMain".into(),
    };
    let wanted = Value::Service {
        start: 2,
        delayed: true,
        running: true,
    };
    let pending = Value::Service {
        start: 2,
        delayed: true,
        running: false,
    };
    let mut store = Ignoring(pending);
    let mut plan = item(TweakId::SysMain, vec![(service, wanted)], Some(3));
    plan.changes[0].from = Value::Service {
        start: 2,
        delayed: true,
        running: false,
    };
    let done = vestigium_core::carry_out(&plan, &mut store);
    assert_eq!(done.results[0].error, None);
}

#[test]
fn guid_reads_and_writes_its_usual_text() {
    let balanced: vestigium_core::Guid = "381b4222-f694-41f0-9685-ff5bb260df2e".parse().unwrap();
    assert_eq!(balanced, SCHEME_BALANCED);
    assert_eq!(balanced.to_string(), "381b4222-f694-41f0-9685-ff5bb260df2e");
    assert_eq!(
        "{381B4222-F694-41F0-9685-FF5BB260DF2E}".parse(),
        Ok(balanced)
    );
    assert!("381b4222".parse::<vestigium_core::Guid>().is_err());
}

fn app_dump_settings() -> (Setting, Setting) {
    let settings = TweakId::AppDumps.settings();
    (settings[0].clone(), settings[2].clone())
}

#[test]
fn an_intent_holds_what_each_setting_held_before_the_write() {
    let (first, second) = app_dump_settings();
    let plan = item(
        TweakId::AppDumps,
        vec![
            (first.clone(), Value::Dword(1)),
            (second.clone(), Value::Dword(25)),
        ],
        None,
    );
    let mut store = FakeStore {
        values: vec![
            (first.clone(), Value::Dword(0)),
            (second.clone(), Value::Absent),
        ],
        fails: None,
        writes: 0,
    };

    let intent = vestigium_core::intent_of(&[&plan], &mut store, 42);

    assert_eq!(intent.ts_ms, 42);
    assert_eq!(store.writes, 0, "recording an intent writes nothing");
    let held: Vec<_> = intent.items[0]
        .settings
        .iter()
        .map(|s| (s.setting.clone(), s.before.clone(), s.to.clone()))
        .collect();
    assert_eq!(
        held,
        vec![
            (first, Some(Value::Dword(0)), Value::Dword(1)),
            (second, Some(Value::Absent), Value::Dword(25)),
        ]
    );
}

#[test]
fn an_intent_survives_the_file_it_is_written_to() {
    let (first, _) = app_dump_settings();
    let intent = vestigium_core::TweakIntent {
        ts_ms: 7,
        items: vec![vestigium_core::IntentItem {
            tweak: TweakId::AppDumps,
            subject: Some("a".into()),
            undo_of: Some(3),
            settings: vec![vestigium_core::IntentSetting {
                setting: first,
                before: None,
                to: Value::Dword(1),
            }],
        }],
    };
    let bytes = postcard::to_allocvec(&intent).unwrap();
    assert_eq!(
        postcard::from_bytes::<vestigium_core::TweakIntent>(&bytes).unwrap(),
        intent
    );
}

fn settled(now: [Value; 2]) -> vestigium_core::TweakChange {
    let (first, second) = app_dump_settings();
    let plan = item(
        TweakId::AppDumps,
        vec![
            (first.clone(), Value::Dword(1)),
            (second.clone(), Value::Dword(25)),
        ],
        None,
    );
    let mut store = FakeStore {
        values: vec![
            (first.clone(), Value::Dword(0)),
            (second.clone(), Value::Dword(10)),
        ],
        fails: None,
        writes: 0,
    };
    let intent = vestigium_core::intent_of(&[&plan], &mut store, 42);
    let [one, two] = now;
    store.values = vec![(first, one), (second, two)];
    let mut results = vestigium_core::settle_interrupted(&intent, &mut store);
    assert_eq!(results.ts_ms, 42);
    results.changes.pop().unwrap()
}

#[test]
fn a_setting_found_at_its_planned_value_is_written_and_can_be_undone() {
    let change = settled([Value::Dword(1), Value::Dword(25)]);

    let errors: Vec<_> = change.results.iter().map(|r| r.error).collect();
    assert_eq!(errors, [Some(vestigium_core::ERROR_INTERRUPTED_WRITTEN); 2]);
    assert!(change.succeeded());
    assert_eq!(change.written().len(), 2);
    assert_eq!(change.written()[0].from, Value::Dword(0));
    assert_eq!(
        change.restore_point,
        Some(vestigium_core::RestorePoint::Unconfirmed)
    );
}

#[test]
fn a_setting_found_as_it_was_is_unchanged_and_cannot_be_undone() {
    let change = settled([Value::Dword(0), Value::Dword(10)]);

    let errors: Vec<_> = change.results.iter().map(|r| r.error).collect();
    assert_eq!(
        errors,
        [Some(vestigium_core::ERROR_INTERRUPTED_UNCHANGED); 2]
    );
    assert!(!change.succeeded());
    assert!(change.written().is_empty());
}

#[test]
fn a_setting_someone_else_changed_is_unknown_and_cannot_be_undone() {
    let change = settled([Value::Dword(9), Value::Dword(25)]);

    assert_eq!(
        change.results[0].error,
        Some(vestigium_core::ERROR_INTERRUPTED_UNKNOWN)
    );
    assert_eq!(
        change.results[1].error,
        Some(vestigium_core::ERROR_INTERRUPTED_WRITTEN)
    );
    assert!(!change.succeeded());
    assert_eq!(
        change.written().len(),
        1,
        "only what is known to be written"
    );
}

#[test]
fn a_setting_that_cannot_be_read_is_unknown() {
    let (first, second) = app_dump_settings();
    let plan = item(
        TweakId::AppDumps,
        vec![(first.clone(), Value::Dword(1)), (second, Value::Dword(25))],
        None,
    );
    let mut store = FakeStore {
        values: Vec::new(),
        fails: None,
        writes: 0,
    };
    let intent = vestigium_core::intent_of(&[&plan], &mut store, 1);
    assert_eq!(intent.items[0].settings[0].before, None);
    store.values.push((first, Value::Dword(1)));

    let results = vestigium_core::settle_interrupted(&intent, &mut store);

    let errors: Vec<_> = results.changes[0].results.iter().map(|r| r.error).collect();
    assert_eq!(errors, [Some(vestigium_core::ERROR_INTERRUPTED_UNKNOWN); 2]);
}
