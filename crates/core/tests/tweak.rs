use vestigium_core::SettingStore as _;
use vestigium_core::health::{
    BootDelay, Concern, DelayKind, Finding, FindingKind, HealthFact, Measure, Reference, Subject,
};
use vestigium_core::{
    Cause, Channel, Culprit, Episode, EpisodeKind, EventMeaning, Hive, OccurrenceKind,
    OccurrenceSummary, SCHEME_BALANCED, SCHEME_POWER_SAVER, Setting, SettingChange, Severity,
    Signal, StartupEntry, StartupList, Status, TWEAKS, TweakId, TweakInputs, Value, WinEvent,
    advise, disabled_approval, helped, undo,
};

const DAY: i64 = 24 * 60 * 60_000;
const NOW: i64 = 1_800_000_000_000;

fn inputs() -> TweakInputs {
    TweakInputs {
        now_ms: NOW,
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
