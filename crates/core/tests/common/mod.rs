#![allow(dead_code)]

use vestigium_core::{
    Channel, CoreLoad, DiskInfo, DiskLoad, EventMeaning, MachineInfo, Media, ProcessSample,
    ProcessTop, Sample, WinEvent, Window,
};

pub const GIB: u64 = 1 << 30;
/// Start of the stall every synthetic window is built around.
pub const T: i64 = 1_000_000;

pub fn calm(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 12.0,
        dpc_pct: 0.5,
        isr_pct: 0.1,
        perf_limit_pct: Some(100.0),
        proc_perf_pct: Some(99.0),
        mem_total_bytes: 16 * GIB,
        mem_avail_bytes: 8 * GIB,
        commit_bytes: 10 * GIB,
        commit_limit_bytes: 32 * GIB,
        hard_faults_per_s: Some(3.0),
        gpu_pct: Some(4.0),
        probe_max_lag_ms: 0.5,
        sampler_lag_ms: 1.0,
        cores: vec![
            CoreLoad {
                cpu_pct: 12.0,
                dpc_pct: 0.5,
                isr_pct: 0.1,
            };
            4
        ],
        disks: vec![disk("0 C:", 4.0, 1.5, 0.1), disk("1 D:", 1.0, 2.0, 0.0)],
    }
}

pub fn disk(name: &str, active_pct: f32, latency_ms: f32, queue_len: f32) -> DiskLoad {
    DiskLoad {
        name: name.to_string(),
        active_pct,
        latency_ms: Some(latency_ms),
        queue_len: Some(queue_len),
        bytes_per_s: Some(1_000.0),
    }
}

pub fn machine() -> MachineInfo {
    MachineInfo {
        logical_cpus: 4,
        mem_total_bytes: 16 * GIB,
        disks: vec![
            DiskInfo {
                name: "0 C:".to_string(),
                media: Media::Ssd,
            },
            DiskInfo {
                name: "1 D:".to_string(),
                media: Media::Hdd,
            },
        ],
    }
}

pub fn process(pid: u32, name: &str, cpu_pct: f32, private_gib: u64, io_bps: f32) -> ProcessSample {
    ProcessSample {
        pid,
        name: name.to_string(),
        cpu_pct,
        private_bytes: private_gib * GIB,
        read_bps: io_bps,
        write_bps: 0.0,
        gpu_pct: 0.0,
    }
}

pub fn ordinary_top(ts_ms: i64) -> ProcessTop {
    ProcessTop {
        ts_ms,
        entries: vec![
            process(10, "browser.exe", 6.0, 2, 50_000.0),
            process(11, "player.exe", 3.0, 1, 1_000.0),
        ],
    }
}

/// A quiet window from 60 s before `T` to 30 s after, one sample per second and a ranking every 5 s.
pub fn quiet_window() -> Window {
    Window {
        from_ms: T - 60_000,
        to_ms: T + 30_000,
        samples: (-60..=30).map(|s| calm(T + s * 1_000)).collect(),
        tops: (-12..=6).map(|k| ordinary_top(T + k * 5_000)).collect(),
        events: Vec::new(),
        hangs: Vec::new(),
        gaps: Vec::new(),
        machine: machine(),
    }
}

/// Changes the samples whose time falls in `[from, to]` relative to `T`, in seconds.
pub fn tweak(window: &mut Window, from_s: i64, to_s: i64, change: impl Fn(&mut Sample)) {
    for sample in &mut window.samples {
        let s = (sample.ts_ms - T).div_euclid(1_000);
        if (from_s..=to_s).contains(&s) {
            change(sample);
        }
    }
}

pub fn event(ts_ms: i64, provider: &str, id: u16, meaning: EventMeaning) -> WinEvent {
    WinEvent {
        ts_ms,
        channel: Channel::System,
        provider: provider.to_string(),
        id,
        record_id: ts_ms as u64,
        level: 2,
        meaning,
        data_xml: String::new(),
    }
}
