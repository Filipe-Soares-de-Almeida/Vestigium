use std::fmt::Write as _;

const MB: f64 = 1024.0 * 1024.0;
const DAY_MS: f64 = 86_400_000.0;
const MINUTE_MS: f64 = 60_000.0;

/// One reading of the measured process.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    pub at_ms: f64,
    pub cpu_time_100ns: u64,
    pub working_set: u64,
    pub private_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub context_switches: u64,
}

/// Limits a process must stay within.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Budgets {
    pub cpu_pct: f64,
    pub working_set_mb: f64,
    pub writes_mb_per_day: f64,
    pub wakeups_per_s: f64,
}

/// The default-mode budgets of the agent.
pub const AGENT: Budgets = Budgets {
    cpu_pct: 0.3,
    working_set_mb: 15.0,
    writes_mb_per_day: 50.0,
    wakeups_per_s: 12.0,
};

/// What a run of observations adds up to.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub minutes: f64,
    pub cpu_avg_pct: f64,
    pub cpu_worst_minute_pct: f64,
    pub ws_avg_mb: f64,
    pub ws_max_mb: f64,
    pub private_avg_mb: f64,
    pub private_max_mb: f64,
    pub wakeups_per_s: f64,
    pub writes_mb_per_day: f64,
    pub reads_mb_per_day: f64,
    pub endpoints: usize,
    pub budgets: Budgets,
}

pub fn report(
    obs: &[Observation],
    logical_cpus: u32,
    endpoints: usize,
    budgets: &Budgets,
) -> Report {
    let (Some(first), Some(last)) = (obs.first(), obs.last()) else {
        return empty(endpoints, budgets);
    };
    let span_ms = last.at_ms - first.at_ms;
    let cpu_pct = |a: &Observation, b: &Observation| {
        let window_ms = b.at_ms - a.at_ms;
        if window_ms <= 0.0 {
            return 0.0;
        }
        let used = b.cpu_time_100ns.saturating_sub(a.cpu_time_100ns) as f64;
        used / (window_ms * 10_000.0 * f64::from(logical_cpus.max(1))) * 100.0
    };
    let per_day = |a: u64, b: u64| {
        if span_ms <= 0.0 {
            0.0
        } else {
            b.saturating_sub(a) as f64 / MB * DAY_MS / span_ms
        }
    };
    let mean = |f: fn(&Observation) -> u64| {
        obs.iter().map(|o| f(o) as f64).sum::<f64>() / obs.len() as f64 / MB
    };
    let max = |f: fn(&Observation) -> u64| obs.iter().map(f).max().unwrap_or(0) as f64 / MB;

    Report {
        minutes: span_ms / MINUTE_MS,
        cpu_avg_pct: cpu_pct(first, last),
        cpu_worst_minute_pct: worst_minute(obs, cpu_pct),
        ws_avg_mb: mean(|o| o.working_set),
        ws_max_mb: max(|o| o.working_set),
        private_avg_mb: mean(|o| o.private_bytes),
        private_max_mb: max(|o| o.private_bytes),
        wakeups_per_s: if span_ms > 0.0 {
            last.context_switches.saturating_sub(first.context_switches) as f64 * 1000.0 / span_ms
        } else {
            0.0
        },
        writes_mb_per_day: per_day(first.write_bytes, last.write_bytes),
        reads_mb_per_day: per_day(first.read_bytes, last.read_bytes),
        endpoints,
        budgets: *budgets,
    }
}

/// Highest CPU share over any one-minute window, or over the whole run when it is shorter.
fn worst_minute(obs: &[Observation], cpu_pct: impl Fn(&Observation, &Observation) -> f64) -> f64 {
    let mut worst = 0.0f64;
    let mut end = 0;
    for (start, a) in obs.iter().enumerate() {
        while end < obs.len() && obs[end].at_ms < a.at_ms + MINUTE_MS {
            end += 1;
        }
        if end == obs.len() {
            if start == 0 {
                worst = cpu_pct(a, &obs[obs.len() - 1]);
            }
            break;
        }
        worst = worst.max(cpu_pct(a, &obs[end]));
    }
    worst
}

fn empty(endpoints: usize, budgets: &Budgets) -> Report {
    Report {
        minutes: 0.0,
        cpu_avg_pct: 0.0,
        cpu_worst_minute_pct: 0.0,
        ws_avg_mb: 0.0,
        ws_max_mb: 0.0,
        private_avg_mb: 0.0,
        private_max_mb: 0.0,
        wakeups_per_s: 0.0,
        writes_mb_per_day: 0.0,
        reads_mb_per_day: 0.0,
        endpoints,
        budgets: *budgets,
    }
}

impl Report {
    /// Names of the budgets this run went over.
    pub fn exceeded(&self) -> Vec<&'static str> {
        let b = &self.budgets;
        [
            ("CPU", self.cpu_avg_pct > b.cpu_pct),
            ("working set", self.ws_max_mb > b.working_set_mb),
            ("disk writes", self.writes_mb_per_day > b.writes_mb_per_day),
            ("wakeups", self.wakeups_per_s > b.wakeups_per_s),
            ("network", self.endpoints > 0),
        ]
        .into_iter()
        .filter_map(|(name, over)| over.then_some(name))
        .collect()
    }

    pub fn to_markdown(&self) -> String {
        let b = &self.budgets;
        let mut md = format!("Measured over {:.1} min.\n\n", self.minutes);
        md.push_str("| Measure | Value | Budget |\n|---|---|---|\n");
        let rows = [
            (
                "CPU average",
                format!("{:.3} %", self.cpu_avg_pct),
                format!("<= {} %", b.cpu_pct),
            ),
            (
                "CPU, worst minute",
                format!("{:.3} %", self.cpu_worst_minute_pct),
                String::new(),
            ),
            (
                "Working set, average",
                format!("{:.1} MB", self.ws_avg_mb),
                String::new(),
            ),
            (
                "Working set, max",
                format!("{:.1} MB", self.ws_max_mb),
                format!("<= {} MB", b.working_set_mb),
            ),
            (
                "Private bytes, average / max",
                format!("{:.1} / {:.1} MB", self.private_avg_mb, self.private_max_mb),
                String::new(),
            ),
            (
                "Wakeups (context switches)",
                format!("{:.1} /s", self.wakeups_per_s),
                format!("<= {} /s", b.wakeups_per_s),
            ),
            (
                "Disk writes",
                format!("{:.1} MB/day", self.writes_mb_per_day),
                format!("<= {} MB/day", b.writes_mb_per_day),
            ),
            (
                "Disk reads",
                format!("{:.1} MB/day", self.reads_mb_per_day),
                String::new(),
            ),
            (
                "Network endpoints",
                self.endpoints.to_string(),
                "0".to_string(),
            ),
        ];
        for (name, value, budget) in rows {
            let _ = writeln!(md, "| {name} | {value} | {budget} |");
        }
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;

    fn at(seconds: f64) -> Observation {
        Observation {
            at_ms: seconds * 1000.0,
            cpu_time_100ns: 0,
            working_set: 8 * MIB,
            private_bytes: 4 * MIB,
            read_bytes: 0,
            write_bytes: 0,
            context_switches: 0,
        }
    }

    #[test]
    fn writes_are_extrapolated_per_day() {
        let mut end = at(60.0);
        end.write_bytes = MIB;
        let r = report(&[at(0.0), end], 8, 0, &AGENT);
        assert!(
            (r.writes_mb_per_day - 1440.0).abs() < 1e-6,
            "{}",
            r.writes_mb_per_day
        );
    }

    #[test]
    fn cpu_is_a_share_of_the_machine() {
        let mut end = at(10.0);
        end.cpu_time_100ns = 2_400_000;
        let r = report(&[at(0.0), end], 8, 0, &AGENT);
        assert!((r.cpu_avg_pct - 0.3).abs() < 1e-9, "{}", r.cpu_avg_pct);
    }

    #[test]
    fn worst_minute_is_the_heaviest_window() {
        let mut obs: Vec<Observation> = (0..=180).map(|s| at(f64::from(s))).collect();
        for (i, o) in obs.iter_mut().enumerate() {
            o.cpu_time_100ns = if i <= 120 {
                0
            } else {
                (i as u64 - 120) * 800_000
            };
        }
        let r = report(&obs, 8, 0, &AGENT);
        assert!(
            (r.cpu_worst_minute_pct - 1.0).abs() < 1e-6,
            "{}",
            r.cpu_worst_minute_pct
        );
        assert!(r.cpu_avg_pct < r.cpu_worst_minute_pct);
    }

    #[test]
    fn wakeups_are_context_switches_per_s() {
        let mut end = at(60.0);
        end.context_switches = 600;
        let r = report(&[at(0.0), end], 8, 0, &AGENT);
        assert!((r.wakeups_per_s - 10.0).abs() < 1e-9);
    }

    #[test]
    fn report_flags_each_exceeded_budget() {
        let mut big = at(60.0);
        big.working_set = 20 * MIB;
        let r = report(&[at(0.0), big], 8, 0, &AGENT);
        assert_eq!(r.exceeded(), vec!["working set"]);

        let r = report(&[at(0.0), at(60.0)], 8, 1, &AGENT);
        assert_eq!(r.exceeded(), vec!["network"]);
    }

    #[test]
    fn markdown_has_one_row_per_measure() {
        let md = report(&[at(0.0), at(60.0)], 8, 0, &AGENT).to_markdown();
        assert!(md.contains("| CPU average |"));
        assert!(md.contains("| Network endpoints |"));
    }

    #[test]
    fn no_observations_give_an_empty_report() {
        assert_eq!(report(&[], 8, 0, &AGENT).cpu_avg_pct, 0.0);
    }
}
