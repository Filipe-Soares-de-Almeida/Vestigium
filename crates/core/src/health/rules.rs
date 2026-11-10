use crate::hwtest::{
    DiagnosticResult, TEST_REACH_DAYS, TestData, TestRun, VolumeResult, combined_surface,
    memory_diagnostics,
};
use crate::shutdown::{SHUTTING_DOWN, power_loss_detail};
use crate::{EventMeaning, Media, NextStep, WinEvent};

use super::{
    Concern, DAY_MS, DelayKind, DiskHealth, Finding, FindingKind, HealthFact, HealthInputs,
    Measure, NVME_WORN_PCT, Reference, SmartAttribute, Subject, WATCHED_ATTRIBUTES,
    failures_between, watched_count,
};

const GB: u64 = 1 << 30;
/// The system volume is low on space below this free share; an open finding closes above the next.
const LOW_SPACE_PCT: f32 = 10.0;
const LOW_SPACE_CLEAR_PCT: f32 = 12.0;
const CRITICAL_SPACE_PCT: f32 = 5.0;
const CRITICAL_SPACE_BYTES: u64 = 5 * GB;
/// Commit charge near the limit; an open finding closes below the healthy share.
const COMMIT_CRITICAL_PCT: f32 = 90.0;
const COMMIT_HEALTHY_PCT: f32 = 80.0;
const MEMORY_DAYS: u32 = 3;
const MEMORY_CLEAR_DAYS: u32 = 1;
const DISPLAY_RESETS: u32 = 2;
const DISPLAY_RESETS_CRITICAL: u32 = 5;
const SLOW_BOOT_MS: u32 = 90_000;
const HEALTHY_BOOT_S: f32 = 60.0;
const DIRTY_RESTARTS: u32 = 2;
const UNFINISHED_SHUTDOWNS: u32 = 2;
const FAILURE_FLOOR: u32 = 3;
/// Blocks of an HDD slow twice in a row, from which its surface asks for attention.
const SLOW_BLOCKS: u64 = 10;
const TREND_DAYS: i64 = 35;
/// Events shown as evidence of one finding, newest first.
const EVENTS_SHOWN: usize = 10;
const DELAYS_SHOWN: usize = 5;

/// Every finding of one health pass, the most worrying first and in table order otherwise.
pub fn evaluate(inputs: &HealthInputs) -> Vec<Finding> {
    let mut found: Vec<Finding> = inputs.disks.iter().filter_map(disk_smart).collect();
    found.extend(low_space(inputs));
    found.extend(page_file(inputs));
    found.extend(memory_often(inputs));
    found.extend(display_resets(inputs));
    found.extend(hardware_errors(inputs));
    found.extend(storage_errors(inputs));
    found.extend(throttling(inputs));
    found.extend(windows_on_hdd(inputs));
    found.extend(two_antivirus(inputs));
    found.extend(slow_boot(inputs));
    found.extend(dirty_restarts(inputs));
    found.extend(shutdown_unfinished(inputs));
    found.extend(failures_rising(inputs));
    found.extend(no_crash_dump(inputs));
    found.extend(memory_test_errors(inputs));
    found.extend(disk_unreadable(inputs));
    found.extend(volume_errors(inputs));
    found.sort_by_key(|f| std::cmp::Reverse(f.concern));
    found
}

fn finding(
    kind: FindingKind,
    subject: Subject,
    concern: Concern,
    value: Measure,
    reference: Reference,
    evidence: Vec<HealthFact>,
    next_steps: Vec<NextStep>,
) -> Finding {
    Finding {
        kind,
        subject,
        concern,
        value,
        reference,
        evidence,
        next_steps,
    }
}

fn event_facts<'a>(events: impl Iterator<Item = &'a WinEvent>) -> Vec<HealthFact> {
    let mut events: Vec<&WinEvent> = events.collect();
    events.sort_by_key(|e| std::cmp::Reverse(e.ts_ms));
    events
        .into_iter()
        .take(EVENTS_SHOWN)
        .map(|e| HealthFact::Event {
            ts_ms: e.ts_ms,
            provider: e.provider.clone(),
            id: e.id,
            meaning: e.meaning,
        })
        .collect()
}

fn matching<'a>(
    inputs: &'a HealthInputs,
    days: i64,
    meaning: EventMeaning,
    ids: &'a [u16],
) -> impl Iterator<Item = &'a WinEvent> {
    inputs
        .events_since(days)
        .filter(move |e| e.meaning == meaning && (ids.is_empty() || ids.contains(&e.id)))
}

fn disk_smart(disk: &DiskHealth) -> Option<Finding> {
    let watched: Vec<SmartAttribute> = disk
        .attributes
        .iter()
        .filter(|a| WATCHED_ATTRIBUTES.contains(&a.id))
        .map(|a| SmartAttribute {
            raw: u64::from(watched_count(a)),
            ..*a
        })
        .filter(|a| a.raw > 0)
        .collect();
    let nvme_critical = disk
        .nvme
        .is_some_and(|n| n.critical_warning != 0 || n.available_spare_pct < n.spare_threshold_pct);
    let nvme_worn = disk
        .nvme
        .is_some_and(|n| n.percentage_used >= NVME_WORN_PCT || n.media_errors > 0);
    let critical = disk.predict_failure || nvme_critical;
    if !critical && !nvme_worn && watched.is_empty() {
        return None;
    }
    let mut evidence = Vec::new();
    if disk.predict_failure {
        evidence.push(HealthFact::PredictFailure);
    }
    evidence.extend(watched.iter().map(|a| HealthFact::Attribute(*a)));
    let (value, reference) = match disk.nvme {
        Some(nvme) => {
            evidence.push(HealthFact::Nvme(nvme));
            (
                Measure::Percent(f32::from(nvme.percentage_used)),
                Reference::AtMost(Measure::Percent(f32::from(NVME_WORN_PCT))),
            )
        }
        None => {
            let highest = watched.iter().map(|a| a.raw).max().unwrap_or(0);
            (
                Measure::Count(u32::try_from(highest).unwrap_or(u32::MAX)),
                Reference::AtMost(Measure::Count(0)),
            )
        }
    };
    let (concern, next_steps) = if critical {
        (
            Concern::Critical,
            vec![NextStep::BackUpNow, NextStep::ReplaceDisk],
        )
    } else {
        (
            Concern::Attention,
            vec![NextStep::BackUpNow, NextStep::RunDiskTest],
        )
    };
    Some(finding(
        FindingKind::DiskSmart,
        Subject::Disk(disk.name.clone()),
        concern,
        value,
        reference,
        evidence,
        next_steps,
    ))
}

fn low_space(inputs: &HealthInputs) -> Option<Finding> {
    let volume = inputs
        .volumes
        .iter()
        .find(|v| v.system && v.total_bytes > 0)?;
    let subject = Subject::Volume(volume.root.clone());
    let pct = volume.free_bytes as f32 * 100.0 / volume.total_bytes as f32;
    let limit = if inputs.is_open(FindingKind::LowSpace, &subject) {
        LOW_SPACE_CLEAR_PCT
    } else {
        LOW_SPACE_PCT
    };
    if pct >= limit {
        return None;
    }
    let concern = if pct < CRITICAL_SPACE_PCT || volume.free_bytes < CRITICAL_SPACE_BYTES {
        Concern::Critical
    } else {
        Concern::Attention
    };
    Some(finding(
        FindingKind::LowSpace,
        subject,
        concern,
        Measure::Percent(pct),
        Reference::AtLeast(Measure::Percent(LOW_SPACE_PCT)),
        vec![HealthFact::Space {
            free_bytes: volume.free_bytes,
            total_bytes: volume.total_bytes,
        }],
        vec![NextStep::FreeDiskSpace],
    ))
}

fn page_file(inputs: &HealthInputs) -> Option<Finding> {
    let files = inputs.page_files.as_ref()?;
    let limit = if inputs.is_open(FindingKind::PageFile, &Subject::Pc) {
        COMMIT_HEALTHY_PCT
    } else {
        COMMIT_CRITICAL_PCT
    };
    let commit_high = inputs.commit_peak_pct_7d.is_some_and(|pct| pct >= limit);
    let exhausted: Vec<&WinEvent> =
        matching(inputs, 7, EventMeaning::LowVirtualMemory, &[]).collect();
    let critical = commit_high || !exhausted.is_empty();
    if !critical && !files.is_empty() {
        return None;
    }
    let mut evidence = if files.is_empty() {
        vec![HealthFact::NoPageFile]
    } else {
        files.iter().cloned().map(HealthFact::PageFile).collect()
    };
    if let Some(pct) = inputs.commit_peak_pct_7d {
        evidence.push(HealthFact::CommitPeak { pct });
    }
    evidence.extend(event_facts(exhausted.into_iter()));
    let (concern, value, reference, mut next_steps) = match (critical, inputs.commit_peak_pct_7d) {
        (true, Some(pct)) => (
            Concern::Critical,
            Measure::Percent(pct),
            Reference::AtMost(Measure::Percent(COMMIT_HEALTHY_PCT)),
            vec![NextStep::CloseMemoryHeavyApp],
        ),
        (true, None) => (
            Concern::Critical,
            Measure::Count(files.len() as u32),
            Reference::AtLeast(Measure::Count(1)),
            vec![NextStep::CloseMemoryHeavyApp],
        ),
        (false, _) => (
            Concern::Attention,
            Measure::Count(0),
            Reference::AtLeast(Measure::Count(1)),
            Vec::new(),
        ),
    };
    next_steps.insert(0, NextStep::LetWindowsManagePageFile);
    Some(finding(
        FindingKind::PageFile,
        Subject::Pc,
        concern,
        value,
        reference,
        evidence,
        next_steps,
    ))
}

fn memory_often(inputs: &HealthInputs) -> Option<Finding> {
    let days = inputs.memory_high_days_7d;
    let open = inputs.is_open(FindingKind::MemoryOften, &Subject::Pc);
    if days < MEMORY_DAYS && !(open && days > MEMORY_CLEAR_DAYS) {
        return None;
    }
    Some(finding(
        FindingKind::MemoryOften,
        Subject::Pc,
        Concern::Attention,
        Measure::Days(days),
        Reference::AtMost(Measure::Days(MEMORY_CLEAR_DAYS)),
        vec![HealthFact::MemoryHighDays { days }],
        vec![NextStep::CloseMemoryHeavyApp],
    ))
}

/// A finding that counts events of one meaning: attention from `attention`, critical from `critical`.
fn counted(
    kind: FindingKind,
    events: Vec<&WinEvent>,
    attention: u32,
    critical: Option<u32>,
    next_steps: Vec<NextStep>,
) -> Option<Finding> {
    let n = events.len() as u32;
    if n < attention {
        return None;
    }
    let concern = if critical.is_some_and(|c| n >= c) {
        Concern::Critical
    } else {
        Concern::Attention
    };
    Some(finding(
        kind,
        Subject::Pc,
        concern,
        Measure::Count(n),
        Reference::AtMost(Measure::Count(0)),
        event_facts(events.into_iter()),
        next_steps,
    ))
}

fn display_resets(inputs: &HealthInputs) -> Option<Finding> {
    let events = matching(inputs, 7, EventMeaning::DisplayReset, &[]).collect();
    counted(
        FindingKind::DisplayResets,
        events,
        DISPLAY_RESETS,
        Some(DISPLAY_RESETS_CRITICAL),
        vec![NextStep::UpdateGpuDriver],
    )
}

fn hardware_errors(inputs: &HealthInputs) -> Option<Finding> {
    let events: Vec<&WinEvent> = matching(inputs, 30, EventMeaning::HardwareError, &[]).collect();
    let fatal = events.iter().any(|e| e.id == 18);
    let mut found = counted(
        FindingKind::HardwareErrors,
        events,
        1,
        None,
        vec![NextStep::CheckHardwareStability],
    )?;
    if fatal {
        found.concern = Concern::Critical;
    }
    Some(found)
}

/// The N of the `\Device\HarddiskN` a disk event names.
fn harddisk_number(data_xml: &str) -> Option<u32> {
    const DEVICE: &str = r"\device\harddisk";
    let lower = data_xml.to_ascii_lowercase();
    let digits = &lower[lower.find(DEVICE)? + DEVICE.len()..];
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    digits[..end].parse().ok()
}

/// Whether a storage event is about one of the PC's disks: it names none, or one the watcher
/// measures. A virtual or unplugged disk is left out.
fn on_pc_disk(inputs: &HealthInputs, event: &WinEvent) -> bool {
    let Some(number) = harddisk_number(&event.data_xml) else {
        return true;
    };
    let mut known = inputs
        .machine
        .disks
        .iter()
        .filter_map(|d| d.name.split_whitespace().next()?.parse::<u32>().ok())
        .peekable();
    known.peek().is_none() || known.any(|n| n == number)
}

fn storage_errors(inputs: &HealthInputs) -> Option<Finding> {
    let bad: Vec<&WinEvent> = matching(inputs, 30, EventMeaning::DiskBadBlock, &[])
        .chain(matching(inputs, 30, EventMeaning::PagingError, &[]))
        .filter(|e| on_pc_disk(inputs, e))
        .collect();
    let resets = matching(inputs, 30, EventMeaning::StorageReset, &[])
        .chain(matching(inputs, 30, EventMeaning::DiskRetry, &[]))
        .filter(|e| on_pc_disk(inputs, e));
    let critical = !bad.is_empty();
    let events: Vec<&WinEvent> = bad.into_iter().chain(resets).collect();
    let next_steps = if critical {
        vec![NextStep::BackUpNow, NextStep::RunDiskTest]
    } else {
        vec![NextStep::UpdateStorageDriver, NextStep::CheckDiskCable]
    };
    let mut found = counted(FindingKind::StorageErrors, events, 1, None, next_steps)?;
    if critical {
        found.concern = Concern::Critical;
    }
    Some(found)
}

fn throttling(inputs: &HealthInputs) -> Option<Finding> {
    let events = matching(inputs, 7, EventMeaning::CpuFirmwareLimit, &[]).collect();
    counted(
        FindingKind::Throttling,
        events,
        1,
        None,
        vec![NextStep::CheckCooling, NextStep::CheckPowerPlan],
    )
}

fn windows_on_hdd(inputs: &HealthInputs) -> Option<Finding> {
    let name = inputs.system_disk.as_ref()?;
    let disk = inputs.machine.disks.iter().find(|d| &d.name == name)?;
    (disk.media == Media::Hdd).then(|| {
        finding(
            FindingKind::WindowsOnHdd,
            Subject::Disk(name.clone()),
            Concern::Attention,
            Measure::Media(Media::Hdd),
            Reference::Is(Measure::Media(Media::Ssd)),
            vec![HealthFact::SystemDisk {
                disk: name.clone(),
                media: Media::Hdd,
            }],
            vec![NextStep::MoveWindowsToSsd],
        )
    })
}

fn two_antivirus(inputs: &HealthInputs) -> Option<Finding> {
    let on: Vec<_> = inputs.antivirus.as_ref()?.iter().filter(|p| p.on).collect();
    (on.len() >= 2).then(|| {
        finding(
            FindingKind::TwoAntivirus,
            Subject::Pc,
            Concern::Attention,
            Measure::Count(on.len() as u32),
            Reference::AtMost(Measure::Count(1)),
            on.into_iter().cloned().map(HealthFact::Product).collect(),
            vec![NextStep::KeepOneAntivirus],
        )
    })
}

fn slow_boot(inputs: &HealthInputs) -> Option<Finding> {
    let report = inputs.boot.as_ref()?;
    let from = inputs.now_ms - 30 * DAY_MS;
    let latest = report
        .boots
        .iter()
        .filter(|b| b.ts_ms >= from)
        .max_by_key(|b| b.ts_ms);
    let mut delays: Vec<_> = report.delays.iter().filter(|d| d.ts_ms >= from).collect();
    let slow = latest.is_some_and(|b| b.boot_ms > SLOW_BOOT_MS);
    if !slow && delays.is_empty() {
        return None;
    }
    delays.sort_by_key(|d| std::cmp::Reverse(d.degradation_ms));
    delays.truncate(DELAYS_SHOWN);
    let value = latest.map_or(Measure::Count(delays.len() as u32), |b| {
        Measure::Seconds(b.boot_ms as f32 / 1000.0)
    });
    let startup_app = delays
        .iter()
        .any(|d| d.kind == DelayKind::App && !d.security);
    let mut evidence: Vec<HealthFact> = latest.map(|b| HealthFact::Boot(*b)).into_iter().collect();
    evidence.extend(delays.into_iter().cloned().map(HealthFact::Delay));
    Some(finding(
        FindingKind::SlowBoot,
        Subject::Pc,
        Concern::Attention,
        value,
        Reference::AtMost(Measure::Seconds(HEALTHY_BOOT_S)),
        evidence,
        if startup_app {
            vec![NextStep::ReviewStartupApps]
        } else {
            Vec::new()
        },
    ))
}

/// Kernel-Power 41 events of the last `days`, split by whether Windows was shutting down.
fn power_losses(inputs: &HealthInputs, days: i64, shutting_down: bool) -> Vec<&WinEvent> {
    matching(inputs, days, EventMeaning::PowerLoss, &[])
        .filter(|e| {
            power_loss_detail(&e.data_xml)
                .is_some_and(|d| (d.power_state == SHUTTING_DOWN) == shutting_down)
        })
        .collect()
}

fn dirty_restarts(inputs: &HealthInputs) -> Option<Finding> {
    let falls = power_losses(inputs, 30, false);
    let blue_screen = falls.iter().any(|e| {
        e.ts_ms >= inputs.now_ms - 7 * DAY_MS
            && power_loss_detail(&e.data_xml).is_some_and(|d| d.bugcheck_code != 0)
    }) || matching(inputs, 7, EventMeaning::BugCheck, &[])
        .next()
        .is_some();
    let n = falls.len() as u32;
    if n < DIRTY_RESTARTS && !blue_screen {
        return None;
    }
    let reports = matching(inputs, 30, EventMeaning::BugCheck, &[]);
    Some(finding(
        FindingKind::DirtyRestarts,
        Subject::Pc,
        if blue_screen {
            Concern::Critical
        } else {
            Concern::Attention
        },
        Measure::Count(n),
        Reference::AtMost(Measure::Count(0)),
        event_facts(falls.into_iter().chain(reports)),
        vec![NextStep::CheckHardwareStability],
    ))
}

fn shutdown_unfinished(inputs: &HealthInputs) -> Option<Finding> {
    counted(
        FindingKind::ShutdownUnfinished,
        power_losses(inputs, 30, true),
        UNFINISHED_SHUTDOWNS,
        None,
        vec![NextStep::WaitForShutdown],
    )
}

fn failures_rising(inputs: &HealthInputs) -> Option<Finding> {
    let now = inputs.now_ms;
    if now - inputs.history_from_ms < TREND_DAYS * DAY_MS {
        return None;
    }
    let week_start = now - 7 * DAY_MS;
    let last_week = failures_between(&inputs.events, week_start, now + 1);
    let before = failures_between(&inputs.events, now - TREND_DAYS * DAY_MS, week_start);
    let weekly_before = before as f32 / ((TREND_DAYS - 7) / 7) as f32;
    if last_week < FAILURE_FLOOR || (last_week as f32) < 2.0 * weekly_before {
        return None;
    }
    Some(finding(
        FindingKind::FailuresRising,
        Subject::Pc,
        Concern::Attention,
        Measure::Count(last_week),
        Reference::AtMost(Measure::Count(weekly_before.round() as u32)),
        vec![HealthFact::Failures {
            last_week,
            weekly_before,
        }],
        Vec::new(),
    ))
}

fn no_crash_dump(inputs: &HealthInputs) -> Option<Finding> {
    let events = matching(inputs, 30, EventMeaning::NoCrashDump, &[]).collect();
    counted(
        FindingKind::NoCrashDump,
        events,
        1,
        None,
        vec![NextStep::LetWindowsManagePageFile],
    )
}

fn recent_tests(inputs: &HealthInputs) -> impl Iterator<Item = &TestRun> {
    let from = inputs.now_ms - TEST_REACH_DAYS * DAY_MS;
    inputs.tests.iter().filter(move |r| r.ended_ms >= from)
}

fn memory_test_errors(inputs: &HealthInputs) -> Option<Finding> {
    let diagnostics = memory_diagnostics(&inputs.events);
    let last_clean = diagnostics
        .iter()
        .filter(|d| d.result == DiagnosticResult::Pass)
        .map(|d| d.ts_ms)
        .max();
    let after_clean = |ts_ms: i64| last_clean.is_none_or(|clean| ts_ms > clean);
    let mut evidence = Vec::new();
    let mut errors = 0u64;
    for run in diagnostics
        .iter()
        .filter(|d| d.result == DiagnosticResult::Errors && after_clean(d.ts_ms))
    {
        let bad_pages = run.detail.map(|d| d.bad_pages);
        errors += bad_pages.unwrap_or(1).max(1);
        evidence.push(HealthFact::MemoryDiagnostic {
            ts_ms: run.ts_ms,
            bad_pages,
        });
    }
    for run in recent_tests(inputs).filter(|r| after_clean(r.ended_ms)) {
        if let TestData::MemoryQuick(quick) = &run.data
            && quick.error_count > 0
        {
            errors += quick.error_count;
            evidence.push(HealthFact::MemoryQuickTest {
                ts_ms: run.ended_ms,
                errors: quick.error_count,
                tested_bytes: quick.tested_bytes,
            });
        }
    }
    if evidence.is_empty() {
        return None;
    }
    Some(finding(
        FindingKind::MemoryTestErrors,
        Subject::Pc,
        Concern::Critical,
        Measure::Count(u32::try_from(errors).unwrap_or(u32::MAX)),
        Reference::AtMost(Measure::Count(0)),
        evidence,
        vec![NextStep::CheckHardwareStability, NextStep::RunMemTest86],
    ))
}

/// Runs grouped by subject, each group oldest first, groups in the order they first appear.
fn by_subject<'a>(runs: impl Iterator<Item = &'a TestRun>) -> Vec<(Subject, Vec<&'a TestRun>)> {
    let mut groups: Vec<(Subject, Vec<&TestRun>)> = Vec::new();
    for run in runs {
        match groups.iter_mut().find(|(s, _)| *s == run.subject) {
            Some((_, group)) => group.push(run),
            None => groups.push((run.subject.clone(), vec![run])),
        }
    }
    for (_, group) in &mut groups {
        group.sort_by_key(|r| r.started_ms);
    }
    groups
}

fn disk_unreadable(inputs: &HealthInputs) -> Vec<Finding> {
    let scans = recent_tests(inputs).filter(|r| matches!(r.data, TestData::DiskSurface(_)));
    let mut found = Vec::new();
    for (subject, runs) in by_subject(scans) {
        let ended_ms = runs.last().map_or(0, |r| r.ended_ms);
        let surfaces: Vec<_> = runs
            .iter()
            .filter_map(|r| match &r.data {
                TestData::DiskSurface(scan) => Some(scan.clone()),
                _ => None,
            })
            .collect();
        let Some(scan) = combined_surface(&surfaces) else {
            continue;
        };
        let slow = scan.slow_blocks();
        let (concern, value, next_steps) = if scan.unreadable_blocks > 0 {
            (
                Concern::Critical,
                scan.unreadable_blocks,
                vec![NextStep::BackUpNow, NextStep::ReplaceDisk],
            )
        } else if scan.media == Media::Hdd && slow >= SLOW_BLOCKS && !scan.outside_io {
            (
                Concern::Attention,
                slow,
                vec![NextStep::BackUpNow, NextStep::CheckDiskCable],
            )
        } else {
            continue;
        };
        found.push(finding(
            FindingKind::DiskUnreadable,
            subject,
            concern,
            Measure::Count(u32::try_from(value).unwrap_or(u32::MAX)),
            Reference::AtMost(Measure::Count(0)),
            vec![HealthFact::SurfaceScan {
                ts_ms: ended_ms,
                unreadable_blocks: scan.unreadable_blocks,
                slow_blocks: slow,
                whole_disk: scan.whole_disk(),
            }],
            next_steps,
        ));
    }
    found
}

fn volume_errors(inputs: &HealthInputs) -> Vec<Finding> {
    let checks = recent_tests(inputs).filter(|r| matches!(r.data, TestData::VolumeCheck(_)));
    by_subject(checks)
        .into_iter()
        .filter_map(|(subject, runs)| {
            let last = runs.last()?;
            let TestData::VolumeCheck(check) = &last.data else {
                return None;
            };
            (check.result == VolumeResult::ProblemsFound).then(|| {
                finding(
                    FindingKind::VolumeErrors,
                    subject,
                    Concern::Attention,
                    Measure::Count(1),
                    Reference::AtMost(Measure::Count(0)),
                    vec![HealthFact::VolumeCheck {
                        ts_ms: last.ended_ms,
                    }],
                    vec![NextStep::RepairVolume, NextStep::BackUpNow],
                )
            })
        })
        .collect()
}
