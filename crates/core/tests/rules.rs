mod common;

use common::{T, event, process, quiet_window, tweak};
use vestigium_core::{
    Cause, Confidence, EventMeaning, Evidence, HangOutcome, Metric, NextStep, ProcessTop, Sample,
    Suspect, Trigger, Window, suspects,
};

type Case = (Vec<Trigger>, Window);

fn freeze(seconds: i64) -> Trigger {
    Trigger::Freeze {
        start_ms: T,
        end_ms: T + seconds * 1_000,
        lag_ms: (seconds * 1_000) as f32,
    }
}

fn find(found: &[Suspect], matches: impl Fn(&Cause) -> bool) -> Option<&Suspect> {
    found.iter().find(|s| matches(&s.cause))
}

fn storage_case() -> Case {
    let mut window = quiet_window();
    window.events.push(event(
        T - 2_000,
        "stornvme",
        129,
        EventMeaning::StorageReset,
    ));
    tweak(&mut window, -2, 1, |s| s.disks[0].latency_ms = 400.0);
    (vec![freeze(2)], window)
}

fn disk_case() -> Case {
    let mut window = quiet_window();
    tweak(&mut window, -5, -1, |s| {
        s.disks[1].latency_ms = 450.0;
        s.disks[1].active_pct = 99.0;
    });
    (vec![freeze(2)], window)
}

fn memory_case() -> Case {
    let mut window = quiet_window();
    tweak(&mut window, -20, 0, |s| s.mem_avail_bytes = common::GIB / 2);
    (vec![freeze(2)], window)
}

fn cpu_case() -> Case {
    let mut window = quiet_window();
    tweak(&mut window, -8, -4, |s| {
        s.cpu_pct = 99.0;
        s.cores.iter_mut().for_each(|c| c.cpu_pct = 99.0);
    });
    window.tops.push(ProcessTop {
        ts_ms: T - 5_000,
        entries: vec![process(20, "builder.exe", 90.0, 1, 0.0)],
    });
    window.tops.sort_by_key(|t| t.ts_ms);
    (vec![freeze(2)], window)
}

fn dpc_case(load: f32) -> Case {
    let mut window = quiet_window();
    tweak(&mut window, 1, 1, |s| s.cores[2].dpc_pct = load);
    (vec![freeze(2)], window)
}

fn gpu_case() -> Case {
    let mut window = quiet_window();
    window.events.push(event(
        T + 1_000,
        "Display",
        4101,
        EventMeaning::DisplayReset,
    ));
    (vec![freeze(2)], window)
}

fn throttling_case() -> Case {
    let mut window = quiet_window();
    tweak(&mut window, -10, -5, |s| s.perf_limit_pct = 20.0);
    (vec![freeze(2)], window)
}

fn app_case() -> Case {
    let hang = Trigger::AppHang {
        pid: 11,
        name: "player.exe".to_string(),
        start_ms: T,
        end_ms: T + 9_000,
        outcome: HangOutcome::Recovered,
    };
    (vec![hang], quiet_window())
}

fn quiet_case() -> Case {
    (vec![freeze(2)], quiet_window())
}

fn run((triggers, window): &Case) -> Vec<Suspect> {
    suspects(triggers, window)
}

#[test]
fn storage_reset_matches() {
    let found = run(&storage_case());
    let storage = find(&found, |c| matches!(c, Cause::Storage { .. })).expect("storage");
    assert_eq!(
        storage.cause,
        Cause::Storage {
            disk: Some("0 C:".to_string())
        }
    );
    assert_eq!(storage.confidence, Confidence::High);
}

#[test]
fn storage_reset_stays_quiet() {
    assert!(find(&run(&quiet_case()), |c| matches!(c, Cause::Storage { .. })).is_none());
}

#[test]
fn hyper_v_129_is_not_a_storage_reset() {
    let (triggers, mut window) = quiet_case();
    window.events.push(event(
        T - 1_000,
        "Microsoft-Windows-Hyper-V-StorageVSP",
        129,
        EventMeaning::StorageReset,
    ));
    let found = suspects(&triggers, &window);
    assert!(find(&found, |c| matches!(c, Cause::Storage { .. })).is_none());
}

#[test]
fn disk_latency_matches() {
    let found = run(&disk_case());
    let disk = find(&found, |c| matches!(c, Cause::DiskIo { .. })).expect("disk");
    assert_eq!(
        disk.cause,
        Cause::DiskIo {
            disk: "1 D:".to_string(),
            hdd: true,
            process: Some("browser.exe".to_string()),
        }
    );
    assert_eq!(disk.confidence, Confidence::High, "an HDD at 99% active");
}

#[test]
fn disk_latency_stays_quiet() {
    let (triggers, mut window) = quiet_case();
    tweak(&mut window, -5, -1, |s| s.disks[1].latency_ms = 150.0);
    let found = suspects(&triggers, &window);
    assert!(find(&found, |c| matches!(c, Cause::DiskIo { .. })).is_none());
}

#[test]
fn memory_matches() {
    let found = run(&memory_case());
    let memory = find(&found, |c| matches!(c, Cause::Memory { .. })).expect("memory");
    assert_eq!(
        memory.cause,
        Cause::Memory {
            process: Some("browser.exe".to_string())
        }
    );
}

#[test]
fn memory_stays_quiet() {
    let (triggers, mut window) = quiet_case();
    tweak(&mut window, -20, 0, |s| s.mem_avail_bytes = 2 * common::GIB);
    let found = suspects(&triggers, &window);
    assert!(find(&found, |c| matches!(c, Cause::Memory { .. })).is_none());
}

#[test]
fn hard_faults_with_memory_to_spare_are_not_memory() {
    let (triggers, mut window) = quiet_case();
    tweak(&mut window, -10, -10, |s| s.hard_faults_per_s = 5_000.0);
    let found = suspects(&triggers, &window);
    assert!(find(&found, |c| matches!(c, Cause::Memory { .. })).is_none());
}

#[test]
fn hard_faults_with_memory_high_are_memory() {
    let (triggers, mut window) = quiet_case();
    let total = window.machine.mem_total_bytes;
    tweak(&mut window, -20, 0, |s| s.mem_avail_bytes = total / 10);
    tweak(&mut window, -10, -10, |s| s.hard_faults_per_s = 5_000.0);
    let found = suspects(&triggers, &window);
    let memory = find(&found, |c| matches!(c, Cause::Memory { .. })).expect("memory");
    assert!(memory.evidence.iter().any(|e| matches!(
        e,
        Evidence::Metric {
            metric: Metric::HardFaultsPerS,
            ..
        }
    )));
}

#[test]
fn cpu_matches() {
    let found = run(&cpu_case());
    let cpu = find(&found, |c| matches!(c, Cause::Cpu { .. })).expect("cpu");
    let Cause::Cpu { processes } = &cpu.cause else {
        unreachable!()
    };
    assert_eq!(processes.first().map(String::as_str), Some("builder.exe"));
}

#[test]
fn cpu_stays_quiet() {
    let (triggers, mut window) = quiet_case();
    tweak(&mut window, -8, -7, |s| {
        s.cores.iter_mut().for_each(|c| c.cpu_pct = 99.0);
    });
    let found = suspects(&triggers, &window);
    assert!(find(&found, |c| matches!(c, Cause::Cpu { .. })).is_none());
}

#[test]
fn cpu_sees_the_sample_after_a_freeze() {
    let (_, mut window) = quiet_case();
    window
        .samples
        .retain(|s| !(T..T + 8_000).contains(&s.ts_ms));
    tweak(&mut window, 8, 8, |s| {
        s.cores.iter_mut().for_each(|c| c.cpu_pct = 100.0);
    });
    let found = suspects(&[freeze(8)], &window);
    assert!(find(&found, |c| matches!(c, Cause::Cpu { .. })).is_some());
}

#[test]
fn cpu_names_programs_not_processes() {
    let (triggers, mut window) = cpu_case();
    window.tops.push(ProcessTop {
        ts_ms: T - 5_000,
        entries: vec![
            process(31, "rustc.exe", 60.0, 1, 0.0),
            process(32, "rustc.exe", 60.0, 1, 0.0),
        ],
    });
    window.tops.sort_by_key(|t| t.ts_ms);

    let found = suspects(&triggers, &window);
    let cpu = find(&found, |c| matches!(c, Cause::Cpu { .. })).expect("cpu");
    let Cause::Cpu { processes } = &cpu.cause else {
        unreachable!()
    };
    assert_eq!(processes[0], "rustc.exe");
    assert_eq!(processes.iter().filter(|p| *p == "rustc.exe").count(), 1);
}

#[test]
fn cpu_sees_a_freeze_its_late_sample_dilutes() {
    let (_, mut window) = quiet_case();
    window
        .samples
        .retain(|s| !(T..T + 3_000).contains(&s.ts_ms));
    let all_cores =
        |pct: f32| move |s: &mut Sample| s.cores.iter_mut().for_each(|c| c.cpu_pct = pct);

    tweak(&mut window, 3, 3, all_cores(80.0));
    let found = suspects(&[freeze(3)], &window);
    assert!(find(&found, |c| matches!(c, Cause::Cpu { .. })).is_some());

    tweak(&mut window, 3, 3, all_cores(30.0));
    let found = suspects(&[freeze(3)], &window);
    assert!(find(&found, |c| matches!(c, Cause::Cpu { .. })).is_none());
}

#[test]
fn dpc_isr_matches() {
    let low = run(&dpc_case(30.0));
    let driver = find(&low, |c| matches!(c, Cause::Driver { .. })).expect("driver");
    assert_eq!(driver.cause, Cause::Driver { core: 2 });
    assert_eq!(driver.confidence, Confidence::Low);
    let high = run(&dpc_case(45.0));
    let driver = find(&high, |c| matches!(c, Cause::Driver { .. })).expect("driver");
    assert_eq!(driver.confidence, Confidence::Medium);
}

#[test]
fn dpc_isr_stays_quiet() {
    let found = run(&dpc_case(22.0));
    assert!(find(&found, |c| matches!(c, Cause::Driver { .. })).is_none());
}

#[test]
fn gpu_driver_matches() {
    let found = run(&gpu_case());
    let gpu = find(&found, |c| *c == Cause::GpuDriver).expect("gpu");
    assert_eq!(gpu.confidence, Confidence::High);
    assert_eq!(gpu.next_steps, vec![NextStep::UpdateGpuDriver]);
}

#[test]
fn gpu_driver_stays_quiet() {
    assert!(find(&run(&quiet_case()), |c| *c == Cause::GpuDriver).is_none());
}

#[test]
fn throttling_matches() {
    let found = run(&throttling_case());
    let limit = find(&found, |c| *c == Cause::Throttling).expect("throttling");
    assert_eq!(limit.confidence, Confidence::Low);

    let (triggers, mut window) = throttling_case();
    window.events.push(event(
        T - 3_000,
        "Microsoft-Windows-Kernel-Processor-Power",
        37,
        EventMeaning::CpuFirmwareLimit,
    ));
    let found = suspects(&triggers, &window);
    let limit = find(&found, |c| *c == Cause::Throttling).expect("throttling");
    assert_eq!(limit.confidence, Confidence::Medium);
}

#[test]
fn throttling_stays_quiet() {
    let (triggers, mut window) = quiet_case();
    tweak(&mut window, -10, -5, |s| s.perf_limit_pct = 97.0);
    let found = suspects(&triggers, &window);
    assert!(find(&found, |c| *c == Cause::Throttling).is_none());
}

#[test]
fn app_matches() {
    let found = run(&app_case());
    assert_eq!(
        found[0].cause,
        Cause::App {
            name: "player.exe".to_string()
        }
    );
    assert!(found[0].evidence.contains(&Evidence::Hang {
        name: "player.exe".to_string(),
        duration_ms: 9_000
    }));
}

#[test]
fn app_stays_quiet() {
    let (mut triggers, window) = app_case();
    triggers.push(freeze(3));
    let found = suspects(&triggers, &window);
    assert!(find(&found, |c| matches!(c, Cause::App { .. })).is_none());
}

#[test]
fn unclear_when_nothing_matches_suggests_sensitive_mode() {
    let found = run(&quiet_case());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].cause, Cause::Unclear);
    assert!(found[0].next_steps.contains(&NextStep::EnableSensitiveMode));
    assert!(!found[0].evidence.is_empty(), "the ordinary signals");
}

#[test]
fn every_suspect_carries_evidence() {
    let cases = [
        storage_case(),
        disk_case(),
        memory_case(),
        cpu_case(),
        dpc_case(30.0),
        gpu_case(),
        throttling_case(),
        app_case(),
    ];
    for case in &cases {
        for suspect in run(case) {
            assert!(
                !suspect.evidence.is_empty(),
                "{:?} has no evidence",
                suspect.cause
            );
        }
    }
}

#[test]
fn suspects_sort_by_confidence() {
    let (triggers, mut window) = storage_case();
    tweak(&mut window, 1, 1, |s| s.cores[2].dpc_pct = 25.0);
    tweak(&mut window, -20, 0, |s| s.mem_avail_bytes = common::GIB / 2);
    let found = suspects(&triggers, &window);
    let order: Vec<(Confidence, &Cause)> = found.iter().map(|s| (s.confidence, &s.cause)).collect();
    assert!(matches!(
        order.as_slice(),
        [
            (Confidence::High, Cause::Storage { .. }),
            (Confidence::Medium, Cause::DiskIo { .. }),
            (Confidence::Medium, Cause::Memory { .. }),
            (Confidence::Low, Cause::Driver { .. }),
        ]
    ));
}

#[test]
fn an_earlier_reported_event_does_not_move_the_focus() {
    let (_, window) = cpu_case();
    let reported = Trigger::Reported {
        event: event(T - 20_000, "stornvme", 153, EventMeaning::DiskRetry),
    };
    let found = suspects(&[reported, freeze(2)], &window);
    let first = found.first().expect("a suspect");
    assert!(
        matches!(first.cause, Cause::Cpu { .. }),
        "{:?}",
        first.cause
    );
}
