mod common;

use common::{T, quiet_window};
use vestigium_core::{
    Cause, Confidence, Evidence, HangOutcome, HangSpan, Metric, Occurrence, OccurrenceKind,
    Severity, Suspect, same_program,
};

fn occurrence(suspects: Vec<Suspect>) -> Occurrence {
    Occurrence {
        kind: OccurrenceKind::Freeze,
        start_ms: T,
        end_ms: T + 3_000,
        severity: Severity::Severe,
        triggers: Vec::new(),
        window: quiet_window(),
        suspects,
        facts: Vec::new(),
        sensitive: false,
        truncated: false,
    }
}

fn suspect(cause: Cause, evidence: Vec<Evidence>) -> Suspect {
    Suspect {
        cause,
        confidence: Confidence::High,
        evidence,
        next_steps: Vec::new(),
    }
}

#[test]
fn a_program_is_involved_by_cause_evidence_or_hang() {
    let by_cause = occurrence(vec![suspect(
        Cause::Cpu {
            processes: vec!["Builder.exe".to_string()],
        },
        Vec::new(),
    )]);
    assert!(by_cause.involves("builder.exe"), "case does not matter");
    assert!(by_cause.involves("Builder"), "nor the .exe ending");

    let by_evidence = occurrence(vec![suspect(
        Cause::Storage { disk: None },
        vec![Evidence::Process {
            pid: 9,
            name: "steam.exe".to_string(),
            metric: Metric::IoBytesPerS,
            value: 1e8,
        }],
    )]);
    assert!(by_evidence.involves("steam.exe"));

    let mut by_hang = occurrence(Vec::new());
    by_hang.window.hangs.push(HangSpan {
        pid: 7,
        name: "Discord.exe".to_string(),
        start_ms: T,
        end_ms: T + 5_000,
        outcome: HangOutcome::Recovered,
    });
    assert!(by_hang.involves("discord.exe"));
}

#[test]
fn a_program_only_in_the_rankings_is_not_involved() {
    let quiet = occurrence(vec![suspect(Cause::Unclear, Vec::new())]);
    assert!(
        !quiet.involves("browser.exe"),
        "being busy nearby is not being named"
    );
}

#[test]
fn program_names_match_without_case_or_ending() {
    assert!(same_program("Chrome.EXE", "chrome"));
    assert!(!same_program("chrome.exe", "chromedriver.exe"));
}
