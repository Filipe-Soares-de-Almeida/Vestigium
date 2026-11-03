use serde::{Deserialize, Serialize};

use crate::xml::element_text;
use crate::{EventMeaning, WinEvent};

/// Use of physical memory a quick test may bring the PC to; the high-memory episode starts at 85%.
pub const QUICK_USE_CAP: f64 = 0.80;
/// Available memory a quick test always leaves.
pub const QUICK_LEAVE_FREE: u64 = 2 << 30;
/// Memory is taken and locked in pieces of this size.
pub const QUICK_CHUNK: u64 = 256 << 20;
/// Less than this is too little of the memory to be worth testing.
pub const QUICK_MIN: u64 = 1 << 30;
/// Lengths a quick test can run for; the middle one is the default.
pub const QUICK_MINUTES: [u32; 3] = [1, 5, 15];
/// Errors kept in detail; the count goes on past them.
pub const ERRORS_KEPT: usize = 32;
/// Events of one diagnostic run are logged together; farther apart they are two runs.
const PAIR_MS: i64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pattern {
    OwnAddress,
    Random,
    /// Two halves filled alike, changed alike by xor, subtraction and multiplication, then compared.
    Compare,
    SolidBits,
    Checkerboard,
    WalkingOnes,
    WalkingZeros,
}

/// One round of the quick test, in order.
pub const PATTERNS: [Pattern; 7] = [
    Pattern::OwnAddress,
    Pattern::Random,
    Pattern::Compare,
    Pattern::SolidBits,
    Pattern::Checkerboard,
    Pattern::WalkingOnes,
    Pattern::WalkingZeros,
];

/// A word that read back different from what was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryError {
    /// Byte offset inside the tested memory; user mode has no physical address.
    pub offset: u64,
    pub pattern: Pattern,
    pub expected: u64,
    pub actual: u64,
}

impl MemoryError {
    pub fn flipped_bits(&self) -> u32 {
        (self.expected ^ self.actual).count_ones()
    }
}

/// What a quick memory test covered and found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryQuick {
    pub total_bytes: u64,
    /// Locked and tested; the rest belonged to Windows and the apps.
    pub tested_bytes: u64,
    /// Full rounds of every pattern.
    pub rounds: u32,
    pub error_count: u64,
    pub errors: Vec<MemoryError>,
    /// Why the test could not run, when it did not.
    pub failure: Option<String>,
}

/// Bytes a quick test takes now: the least of use up to `QUICK_USE_CAP`, available memory less
/// `QUICK_LEAVE_FREE`, and half of it, in whole pieces; `None` below `QUICK_MIN`.
pub fn quick_test_size(total_bytes: u64, available_bytes: u64) -> Option<u64> {
    let in_use = total_bytes.saturating_sub(available_bytes);
    let to_cap = ((total_bytes as f64 * QUICK_USE_CAP) as u64).saturating_sub(in_use);
    let size = to_cap
        .min(available_bytes.saturating_sub(QUICK_LEAVE_FREE))
        .min(available_bytes / 2);
    let size = size / QUICK_CHUNK * QUICK_CHUNK;
    (size >= QUICK_MIN).then_some(size)
}

/// How a run of the Windows Memory Diagnostic ended, from the least to the most worrying.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticResult {
    Pass,
    Cancelled,
    Incomplete,
    Errors,
}

/// What event 1101 or 1102 says the diagnostic covered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticDetail {
    pub memory_mb: u32,
    /// 10 in the standard mode.
    pub test_type: u32,
    pub duration_s: u32,
    pub test_count: u32,
    pub pages_tested: u64,
    pub pages_untested: u64,
    pub bad_pages: u64,
}

impl DiagnosticDetail {
    /// Share of the pages the diagnostic reached, from 0 to 1.
    pub fn coverage(&self) -> f64 {
        let all = self.pages_tested + self.pages_untested;
        if all == 0 {
            0.0
        } else {
            self.pages_tested as f64 / all as f64
        }
    }
}

/// One run of the Windows Memory Diagnostic, read from the System log.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryDiagnostic {
    pub ts_ms: i64,
    pub result: DiagnosticResult,
    /// Absent when only the summary event was logged.
    pub detail: Option<DiagnosticDetail>,
}

fn result_of(id: u16) -> Option<DiagnosticResult> {
    match id {
        1101 | 1201 => Some(DiagnosticResult::Pass),
        1102 | 1202 => Some(DiagnosticResult::Errors),
        1103 => Some(DiagnosticResult::Cancelled),
        1104 => Some(DiagnosticResult::Incomplete),
        _ => None,
    }
}

fn detail(data_xml: &str) -> Option<DiagnosticDetail> {
    let number = |name: &str| element_text(data_xml, name)?.parse::<u64>().ok();
    let small = |name: &str| number(name).map(|n| u32::try_from(n).unwrap_or(u32::MAX));
    Some(DiagnosticDetail {
        memory_mb: small("MemorySize").unwrap_or(0),
        test_type: small("TestType").unwrap_or(0),
        duration_s: small("TestDuration").unwrap_or(0),
        test_count: small("TestCount").unwrap_or(0),
        pages_tested: number("NumPagesTested")?,
        pages_untested: number("NumPagesUnTested").unwrap_or(0),
        bad_pages: number("NumBadPages").unwrap_or(0),
    })
}

/// The diagnostic runs in these events, oldest first; the events one run logs together merge.
pub fn memory_diagnostics(events: &[WinEvent]) -> Vec<MemoryDiagnostic> {
    let mut events: Vec<&WinEvent> = events
        .iter()
        .filter(|e| e.meaning == EventMeaning::MemoryDiagnostic)
        .collect();
    events.sort_by_key(|e| (e.ts_ms, e.record_id));
    let mut runs: Vec<MemoryDiagnostic> = Vec::new();
    for event in events {
        let Some(mut result) = result_of(event.id) else {
            continue;
        };
        let detail = detail(&event.data_xml);
        if detail.is_some_and(|d| d.bad_pages > 0) {
            result = DiagnosticResult::Errors;
        }
        match runs.last_mut() {
            Some(last) if event.ts_ms - last.ts_ms <= PAIR_MS => {
                last.result = last.result.max(result);
                last.detail = last.detail.or(detail);
            }
            _ => runs.push(MemoryDiagnostic {
                ts_ms: event.ts_ms,
                result,
                detail,
            }),
        }
    }
    runs
}
