use std::collections::{HashSet, VecDeque};

use watchmypc_core::ProcessCounters;

/// Every process started, directly or not, by `root`, in breadth-first order.
pub fn descendants(procs: &[ProcessCounters], root: u32) -> Vec<u32> {
    let mut seen = HashSet::from([root]);
    let mut found = Vec::new();
    let mut queue = VecDeque::from([root]);
    while let Some(parent) = queue.pop_front() {
        let Some(parent_info) = procs.iter().find(|p| p.pid == parent) else {
            continue;
        };
        for child in procs.iter().filter(|p| {
            // A child created before its parent points at an older process that reused the PID.
            p.parent_pid == parent && p.create_time >= parent_info.create_time
        }) {
            if seen.insert(child.pid) {
                found.push(child.pid);
                queue.push_back(child.pid);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, parent_pid: u32, create_time: i64) -> ProcessCounters {
        ProcessCounters {
            pid,
            parent_pid,
            name: format!("p{pid}.exe"),
            create_time,
            cpu_time_100ns: 0,
            working_set: 0,
            private_bytes: 0,
            read_bytes: 0,
            write_bytes: 0,
            threads: 1,
            context_switches: 0,
        }
    }

    #[test]
    fn descendants_follow_the_tree() {
        let procs = [
            proc(10, 1, 100),
            proc(11, 10, 110),
            proc(12, 11, 120),
            proc(13, 1, 130),
        ];
        assert_eq!(descendants(&procs, 10), vec![11, 12]);
    }

    #[test]
    fn descendants_ignore_cycles() {
        let procs = [proc(10, 12, 100), proc(11, 10, 110), proc(12, 11, 120)];
        assert_eq!(descendants(&procs, 10), vec![11, 12]);
    }

    #[test]
    fn older_process_is_not_a_child_of_a_reused_pid() {
        let procs = [proc(10, 1, 500), proc(11, 10, 100)];
        assert!(descendants(&procs, 10).is_empty());
    }
}
