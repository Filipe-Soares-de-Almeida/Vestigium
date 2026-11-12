//! Writes days of plausible history into a new database, for measuring the window:
//! `seed <path-to-db> [days]`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use vestigium_core::{
    CoreLoad, Culprit, DiskLoad, Episode, EpisodeKind, Gap, GapReason, ProcessSample, ProcessTop,
    Retention, Sample,
};
use vestigium_store::{Batch, Batcher, Store};

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;
const NIGHT_MS: i64 = 7 * HOUR_MS;
const CORES: usize = 8;
const GIB: u64 = 1 << 30;
const TOP_EVERY_S: i64 = 5;
const PROGRAMS: [&str; 12] = [
    "msedge.exe",
    "Code.exe",
    "Discord.exe",
    "explorer.exe",
    "dwm.exe",
    "Spotify.exe",
    "steam.exe",
    "OneDrive.exe",
    "svchost.exe",
    "SearchHost.exe",
    "Teams.exe",
    "Vestigium.exe",
];

/// Xorshift noise with a fixed seed, so every run writes the same history.
struct Noise(u64);

impl Noise {
    /// Next value in 0 to 1.
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// A stretch of one day under a load, and the episode it leaves when it is pressure.
struct Stretch {
    from_h: f32,
    minutes: i64,
    cpu: f32,
    mem_used: f32,
    disk: f32,
    episode: Option<EpisodeKind>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path: PathBuf = args.next().ok_or("usage: seed <path-to-db> [days]")?.into();
    let days: i64 = args.next().and_then(|d| d.parse().ok()).unwrap_or(7);
    if path.exists() {
        return Err("the database already exists; seed only writes a new one".into());
    }
    let now_ms = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let first_day_ms = now_ms - now_ms.rem_euclid(DAY_MS) - (days - 1) * DAY_MS;
    let samples_from = now_ms - i64::from(Retention::default().samples_hours) * HOUR_MS;

    let mut store = Store::open(&path)?;
    let mut noise = Noise(0x9e37_79b9_7f4a_7c15);
    let mut batcher = Batcher::new(Duration::from_secs(60));
    let mut written = 0usize;
    for day in 0..days {
        let day_ms = first_day_ms + day * DAY_MS;
        if day_ms >= now_ms {
            break;
        }
        let stretches = stretches(day);
        let awake_ms = day_ms + NIGHT_MS;
        let mut batch = Batch {
            gaps: vec![Gap {
                start_ms: day_ms,
                end_ms: awake_ms.min(now_ms),
                reason: GapReason::Suspended,
            }],
            ..Batch::default()
        };
        let mut ts = awake_ms;
        while ts < (day_ms + DAY_MS).min(now_ms) {
            let hour = (ts - day_ms) as f32 / HOUR_MS as f32;
            let stretch = stretches
                .iter()
                .find(|s| hour >= s.from_h && hour < s.from_h + s.minutes as f32 / 60.0);
            batcher.push_sample(sample(ts, stretch, &mut noise));
            if ts >= samples_from && (ts / 1000) % TOP_EVERY_S == 0 {
                batcher.push_processes(top(ts, stretch, &mut noise));
            }
            written += 1;
            ts += 1000;
            if batcher.due(ts) {
                batch.segments.extend(batcher.take());
            }
        }
        batch.segments.extend(batcher.take_final());
        store.append_batch(&batch)?;
        for s in &stretches {
            let start_ms = day_ms + (s.from_h * HOUR_MS as f32) as i64;
            let end_ms = start_ms + s.minutes * 60_000;
            if let Some(kind) = s.episode.clone()
                && end_ms <= now_ms
            {
                store.insert_episode(&Episode {
                    kind,
                    start_ms,
                    end_ms,
                    peak: s.cpu.max(s.mem_used).max(s.disk),
                    culprit: Some(Culprit {
                        pid: 4200,
                        name: "Code.exe".to_string(),
                        value: f64::from(s.cpu),
                    }),
                    truncated: false,
                })?;
            }
        }
    }
    let pruned = store.prune(now_ms, &Retention::default())?;
    println!("{written} seconds over {days} days written; pruned {pruned:?}");
    Ok(())
}

/// Busy stretches of a day, with one pressure episode most days.
fn stretches(day: i64) -> Vec<Stretch> {
    let mut list = vec![
        Stretch {
            from_h: 10.0 + (day % 3) as f32,
            minutes: 40,
            cpu: 55.0,
            mem_used: 58.0,
            disk: 20.0,
            episode: None,
        },
        Stretch {
            from_h: 15.5,
            minutes: 25,
            cpu: 35.0,
            mem_used: 55.0,
            disk: 70.0,
            episode: None,
        },
    ];
    let pressure = match day % 3 {
        0 => Stretch {
            from_h: 14.0,
            minutes: 4,
            cpu: 99.0,
            mem_used: 60.0,
            disk: 15.0,
            episode: Some(EpisodeKind::CpuSaturated),
        },
        1 => Stretch {
            from_h: 20.0,
            minutes: 3,
            cpu: 30.0,
            mem_used: 91.0,
            disk: 25.0,
            episode: Some(EpisodeKind::MemoryHigh),
        },
        _ => Stretch {
            from_h: 17.0,
            minutes: 2,
            cpu: 20.0,
            mem_used: 50.0,
            disk: 100.0,
            episode: Some(EpisodeKind::DiskBusy {
                disk: "0 C:".to_string(),
            }),
        },
    };
    list.push(pressure);
    list
}

fn sample(ts_ms: i64, stretch: Option<&Stretch>, noise: &mut Noise) -> Sample {
    let (cpu, mem_used, disk) = stretch.map_or((8.0, 42.0, 2.0), |s| (s.cpu, s.mem_used, s.disk));
    let jitter = |n: &mut Noise, base: f32, spread: f32| {
        (base + (n.next() - 0.5) * spread).clamp(0.0, 100.0)
    };
    let cpu_pct = jitter(noise, cpu, 12.0);
    let mem_total_bytes = 32 * GIB;
    let used = jitter(noise, mem_used, 2.0) / 100.0;
    Sample {
        ts_ms,
        cpu_pct,
        dpc_pct: jitter(noise, 0.6, 0.8),
        isr_pct: jitter(noise, 0.4, 0.6),
        perf_limit_pct: Some(100.0),
        proc_perf_pct: Some(jitter(noise, 100.0, 8.0)),
        mem_total_bytes,
        mem_avail_bytes: ((1.0 - used) * mem_total_bytes as f32) as u64,
        commit_bytes: (used * 1.2 * mem_total_bytes as f32) as u64,
        commit_limit_bytes: 44 * GIB,
        hard_faults_per_s: Some(jitter(noise, 40.0, 60.0)),
        gpu_pct: Some(jitter(noise, 4.0, 6.0)),
        probe_max_lag_ms: jitter(noise, 0.8, 0.6),
        sampler_lag_ms: jitter(noise, 0.1, 0.1),
        cores: (0..CORES)
            .map(|_| CoreLoad {
                cpu_pct: jitter(noise, cpu_pct, 20.0),
                dpc_pct: jitter(noise, 0.6, 1.2),
                isr_pct: jitter(noise, 0.4, 0.8),
            })
            .collect(),
        disks: vec![
            DiskLoad {
                name: "0 C:".to_string(),
                active_pct: jitter(noise, disk, 6.0),
                latency_ms: Some(jitter(noise, 0.6, 0.6)),
                queue_len: Some(jitter(noise, disk / 25.0, 1.0)),
                bytes_per_s: Some(jitter(noise, disk, 10.0) * 400_000.0),
            },
            DiskLoad {
                name: "1 D:".to_string(),
                active_pct: jitter(noise, 0.5, 1.0),
                latency_ms: Some(0.0),
                queue_len: Some(0.0),
                bytes_per_s: Some(0.0),
            },
        ],
    }
}

fn top(ts_ms: i64, stretch: Option<&Stretch>, noise: &mut Noise) -> ProcessTop {
    let busy = stretch.map_or(6.0, |s| s.cpu * 0.6);
    ProcessTop {
        ts_ms,
        entries: PROGRAMS
            .iter()
            .enumerate()
            .map(|(rank, name)| {
                let share = busy / (rank as f32 + 1.5);
                ProcessSample {
                    pid: 1000 + rank as u32 * 4,
                    name: (*name).to_string(),
                    cpu_pct: share * (0.8 + noise.next() * 0.4),
                    private_bytes: (300 - rank as u64 * 20) << 20,
                    read_bps: noise.next() * 200_000.0,
                    write_bps: noise.next() * 100_000.0,
                    gpu_pct: if rank < 3 { noise.next() * 5.0 } else { 0.0 },
                }
            })
            .collect(),
    }
}
