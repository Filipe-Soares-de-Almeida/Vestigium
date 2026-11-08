use std::path::Path;

use serde::Serialize;
use vestigium_collect::{ProcessSnapshotter, image_path, run_as, signature};
use vestigium_core::{RunAs, Signature, same_program};

/// The Unix epoch as a Windows FILETIME, in 100 ns units.
const EPOCH_100NS: i64 = 116_444_736_000_000_000;
const SNAPSHOT_BUFFER: usize = 512 * 1024;

/// What a program's page shows about it beyond its use: its file, its user and its processes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProcessFactsDto {
    /// When the process started, Unix epoch milliseconds.
    pub started_ms: Option<i64>,
    /// Summed over every process of the program.
    pub threads: Option<u32>,
    pub processes: Option<u32>,
    pub path: Option<String>,
    pub run_as: RunAs,
    pub signature: Signature,
}

/// Reads the facts of process `pid` and of every process named like it, now.
pub fn facts(pid: u32, name: &str) -> ProcessFactsDto {
    let snapshot = ProcessSnapshotter::with_capacity(SNAPSHOT_BUFFER)
        .snapshot()
        .ok();
    let own: Vec<_> = snapshot
        .iter()
        .flatten()
        .filter(|p| same_program(&p.name, name))
        .collect();
    let path = image_path(pid);
    ProcessFactsDto {
        started_ms: own
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| (p.create_time - EPOCH_100NS) / 10_000),
        threads: snapshot
            .as_ref()
            .map(|_| own.iter().map(|p| p.threads).sum()),
        processes: snapshot.as_ref().map(|_| own.len() as u32),
        signature: path
            .as_deref()
            .map_or(Signature::Unknown, |p| signature(Path::new(p))),
        run_as: run_as(pid),
        path,
    }
}
