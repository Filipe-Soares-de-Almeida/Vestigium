//! Measures a running process against the Argos budgets.
//!
//! `argos-budget watch --name <exe> --minutes <m>` and `argos-budget tree --pid <pid>`.

mod net;
mod report;
mod tree;

use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use argos_collect::{ProcessSnapshotter, logical_cpus, monotonic_ms};
use argos_core::ProcessCounters;

use crate::report::{AGENT, Observation, report};

const NET_EVERY: usize = 10;
const MB: f64 = 1024.0 * 1024.0;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };
    let result = match args.first().map(String::as_str) {
        Some("watch") => match (
            value("--name"),
            value("--minutes").and_then(|m| m.parse().ok()),
        ) {
            (Some(name), Some(minutes)) => watch(&name, minutes),
            _ => Err(usage()),
        },
        Some("tree") => match value("--pid").and_then(|p| p.parse().ok()) {
            Some(pid) => tree(pid),
            None => Err(usage()),
        },
        _ => Err(usage()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn usage() -> String {
    "usage: argos-budget watch --name <exe> --minutes <m> | tree --pid <pid>".to_string()
}

/// Samples the process once a second; true when it stayed within every budget.
fn watch(name: &str, minutes: u64) -> Result<bool, String> {
    let mut snapshotter = ProcessSnapshotter::with_capacity(512 * 1024);
    let snapshot = |s: &mut ProcessSnapshotter| s.snapshot().map_err(|e| e.to_string());
    let target = snapshot(&mut snapshotter)?
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("no running process named {name}"))?;

    let mut observations = Vec::new();
    let mut endpoints = 0;
    for tick in 0..=minutes * 60 {
        if tick > 0 {
            thread::sleep(Duration::from_secs(1));
        }
        let at_ms = monotonic_ms();
        let Some(p) = snapshot(&mut snapshotter)?
            .into_iter()
            .find(|p| p.pid == target.pid && p.create_time == target.create_time)
        else {
            eprintln!("process {} exited after {tick} s", target.pid);
            break;
        };
        observations.push(observe(at_ms, &p));
        if (tick as usize).is_multiple_of(NET_EVERY) {
            endpoints = endpoints.max(net::endpoints_owned_by(target.pid)?);
        }
    }

    let report = report(&observations, logical_cpus(), endpoints, &AGENT);
    print!("{}", report.to_markdown());
    let exceeded = report.exceeded();
    if !exceeded.is_empty() {
        println!("\nOver budget: {}", exceeded.join(", "));
    }
    Ok(exceeded.is_empty())
}

fn observe(at_ms: f64, p: &ProcessCounters) -> Observation {
    Observation {
        at_ms,
        cpu_time_100ns: p.cpu_time_100ns,
        working_set: p.working_set,
        private_bytes: p.private_bytes,
        read_bytes: p.read_bytes,
        write_bytes: p.write_bytes,
        context_switches: p.context_switches,
    }
}

/// Prints the process, everything it started, and their working sets.
fn tree(pid: u32) -> Result<bool, String> {
    let procs = ProcessSnapshotter::with_capacity(512 * 1024)
        .snapshot()
        .map_err(|e| e.to_string())?;
    let root = procs
        .iter()
        .find(|p| p.pid == pid)
        .ok_or_else(|| format!("no process with pid {pid}"))?;
    let mut members = vec![root];
    members.extend(
        tree::descendants(&procs, pid)
            .into_iter()
            .filter_map(|child| procs.iter().find(|p| p.pid == child)),
    );
    println!("| PID | Name | Working set | Private |\n|---|---|---|---|");
    for p in &members {
        println!(
            "| {} | {} | {:.1} MB | {:.1} MB |",
            p.pid,
            p.name,
            p.working_set as f64 / MB,
            p.private_bytes as f64 / MB
        );
    }
    let working_set: u64 = members.iter().map(|p| p.working_set).sum();
    let private: u64 = members.iter().map(|p| p.private_bytes).sum();
    println!(
        "| | **{} processes** | **{:.1} MB** | **{:.1} MB** |",
        members.len(),
        working_set as f64 / MB,
        private as f64 / MB
    );
    Ok(true)
}
