use serde::{Deserialize, Serialize};

use crate::rules::{avail_pct, process_value, worst_disk};
use crate::{
    EventMeaning, GapReason, HangOutcome, HangSpan, Metric, Sample, Trigger, Window, classify,
    threshold,
};

/// One step of what happened; the UI words it in the reader's language.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FactKind {
    FreezeStarted {
        duration_ms: f32,
    },
    FreezeEnded,
    Stutter {
        count: u16,
        worst_ms: f32,
    },
    AppHung {
        name: String,
    },
    AppRecovered {
        name: String,
    },
    AppClosedWhileHung {
        name: String,
    },
    Event {
        meaning: EventMeaning,
        provider: String,
        id: u16,
    },
    /// A metric went past its threshold; `value` is the worst it reached in the window.
    Crossed {
        metric: Metric,
        value: f32,
        threshold: f32,
        subject: Option<String>,
    },
    Busiest {
        metric: Metric,
        name: String,
        value: f64,
    },
    Gap {
        reason: GapReason,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub at_ms: i64,
    pub kind: FactKind,
}

/// The sequence of what happened in the window, in time order, with one crossing per metric.
pub fn facts(triggers: &[Trigger], window: &Window) -> Vec<Fact> {
    let mut facts = Vec::new();
    let mut hangs: Vec<HangSpan> = window.hangs.clone();
    for trigger in triggers {
        match trigger {
            Trigger::Freeze {
                start_ms,
                end_ms,
                lag_ms,
            } => {
                facts.push(fact(
                    *start_ms,
                    FactKind::FreezeStarted {
                        duration_ms: *lag_ms,
                    },
                ));
                facts.push(fact(*end_ms, FactKind::FreezeEnded));
            }
            Trigger::Stutter {
                start_ms,
                count,
                worst_ms,
                ..
            } => facts.push(fact(
                *start_ms,
                FactKind::Stutter {
                    count: *count,
                    worst_ms: *worst_ms,
                },
            )),
            Trigger::AppHang {
                pid,
                name,
                start_ms,
                end_ms,
                outcome,
            } => {
                if !hangs
                    .iter()
                    .any(|h| h.pid == *pid && h.start_ms == *start_ms)
                {
                    hangs.push(HangSpan {
                        pid: *pid,
                        name: name.clone(),
                        start_ms: *start_ms,
                        end_ms: *end_ms,
                        outcome: *outcome,
                    });
                }
            }
            Trigger::Reported { .. } => {}
        }
    }
    for hang in hangs {
        facts.push(fact(
            hang.start_ms,
            FactKind::AppHung {
                name: hang.name.clone(),
            },
        ));
        if hang.end_ms <= window.to_ms {
            let name = hang.name;
            facts.push(fact(
                hang.end_ms,
                match hang.outcome {
                    HangOutcome::Recovered => FactKind::AppRecovered { name },
                    HangOutcome::Closed => FactKind::AppClosedWhileHung { name },
                },
            ));
        }
    }
    for event in &window.events {
        facts.push(fact(
            event.ts_ms,
            FactKind::Event {
                meaning: classify(event.channel, &event.provider, event.id)
                    .unwrap_or(event.meaning),
                provider: event.provider.clone(),
                id: event.id,
            },
        ));
    }
    for metric in CROSSINGS {
        facts.extend(crossing(*metric, window));
    }
    for metric in [Metric::CpuPct, Metric::PrivateBytes, Metric::IoBytesPerS] {
        facts.extend(busiest(metric, window));
    }
    for gap in &window.gaps {
        facts.push(fact(gap.start_ms, FactKind::Gap { reason: gap.reason }));
    }
    facts.sort_by_key(|f| f.at_ms);
    facts
}

const CROSSINGS: &[Metric] = &[
    Metric::CpuPct,
    Metric::MemAvailPct,
    Metric::HardFaultsPerS,
    Metric::DiskLatencyMs,
    Metric::DiskQueue,
    Metric::CoreDpcIsrPct,
    Metric::GpuPct,
    Metric::PerfLimitPct,
];

fn fact(at_ms: i64, kind: FactKind) -> Fact {
    Fact { at_ms, kind }
}

/// The worst value of `metric` in one sample, with the disk or core it belongs to.
fn reading(sample: &Sample, metric: Metric, mem_total: u64) -> Option<(f32, Option<String>)> {
    match metric {
        Metric::CpuPct => Some((sample.cpu_pct, None)),
        Metric::MemAvailPct => avail_pct(sample, mem_total).map(|v| (v, None)),
        Metric::HardFaultsPerS => Some((sample.hard_faults_per_s, None)),
        Metric::DiskLatencyMs => {
            worst_disk([sample], |d| d.latency_ms).map(|(_, disk, v)| (v, Some(disk)))
        }
        Metric::DiskQueue => {
            worst_disk([sample], |d| d.queue_len).map(|(_, disk, v)| (v, Some(disk)))
        }
        Metric::CoreDpcIsrPct => sample
            .cores
            .iter()
            .enumerate()
            .map(|(core, c)| (c.dpc_pct + c.isr_pct, Some(core.to_string())))
            .max_by(|a, b| a.0.total_cmp(&b.0)),
        Metric::GpuPct => sample.gpu_pct.map(|v| (v, None)),
        Metric::PerfLimitPct => Some((sample.perf_limit_pct, None)),
        _ => None,
    }
}

/// When `metric` first went past its threshold, and the worst it reached.
fn crossing(metric: Metric, window: &Window) -> Option<Fact> {
    let limit = threshold(metric);
    let below = metric == Metric::MemAvailPct;
    let past = |v: f32| if below { v < limit } else { v >= limit };
    let worse = |a: f32, b: f32| if below { a < b } else { a > b };
    let mut first: Option<(i64, Option<String>)> = None;
    let mut worst: Option<f32> = None;
    for sample in &window.samples {
        let Some((value, subject)) = reading(sample, metric, window.machine.mem_total_bytes) else {
            continue;
        };
        if !past(value) {
            continue;
        }
        first.get_or_insert((sample.ts_ms, subject));
        if worst.is_none_or(|w| worse(value, w)) {
            worst = Some(value);
        }
    }
    let (at_ms, subject) = first?;
    Some(fact(
        at_ms,
        FactKind::Crossed {
            metric,
            value: worst?,
            threshold: limit,
            subject,
        },
    ))
}

/// The process with the highest `metric` in any ranking of the window.
fn busiest(metric: Metric, window: &Window) -> Option<Fact> {
    let (at_ms, name, value) = window
        .tops
        .iter()
        .flat_map(|t| t.entries.iter().map(move |p| (t.ts_ms, p)))
        .map(|(ts_ms, p)| (ts_ms, &p.name, process_value(p, metric)))
        .filter(|(_, _, value)| *value > 0.0)
        .max_by(|a, b| a.2.total_cmp(&b.2))?;
    Some(fact(
        at_ms,
        FactKind::Busiest {
            metric,
            name: name.clone(),
            value,
        },
    ))
}
