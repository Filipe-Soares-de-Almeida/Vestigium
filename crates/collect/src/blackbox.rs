use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};

use vestigium_core::{DiskLoad, ProcessSample, ProcessTop, Sample};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, OPEN_ALWAYS,
};
use windows::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_WRITE, FlushViewOfFile, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
    PAGE_READWRITE, UnmapViewOfFile,
};
use windows::core::HSTRING;

use crate::CollectError;
use crate::timer::wall_ms;

/// Records the ring holds: five minutes of seconds plus three processes every five seconds, the
/// longest the history can go unwritten.
pub const RING_RECORDS: usize = 480;
const RECORD: usize = 64;
const HEADER: usize = 64;
const RING_BYTES: usize = 32 * 1024;
const _: () = assert!(HEADER + RING_RECORDS * RECORD <= RING_BYTES);
const MAGIC: [u8; 4] = *b"VBXR";
const VERSION: u8 = 1;
const WATCHING: u8 = 0;
const CLOSED: u8 = 1;
const KIND_SAMPLE: u8 = 1;
const KIND_PROCESS: u8 = 2;
const MIB: u64 = 1024 * 1024;
/// The earliest time a record may carry: 2020-01-01 UTC, in Unix milliseconds.
const EARLIEST_MS: i64 = 1_577_836_800_000;
/// How far past the clock a record's time may be.
const LATEST_AHEAD_MS: i64 = 24 * 60 * 60_000;
/// Processes kept from each ranking: the busiest by CPU, by memory and by disk.
const TOP_KEPT: usize = 3;

/// What a ring file held when it was opened.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RingContents {
    /// The agent that wrote it marked a clean exit, or there was nothing to read.
    pub clean: bool,
    pub samples: Vec<Sample>,
    pub tops: Vec<ProcessTop>,
    /// Set when the file could not be read as a ring of this version.
    pub warning: Option<String>,
}

/// The last five minutes in compact records, in a mapped file that outlives the process.
/// Writes land in the page cache at once; `flush` puts them on disk.
#[derive(Debug)]
pub struct BlackboxRing {
    file: HANDLE,
    mapping: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    next: usize,
    /// Where the bytes of a stop not yet settled wait while the ring runs again.
    pending: PathBuf,
}

impl BlackboxRing {
    /// Opens or creates the ring at `path`, returns what it held, and starts it over as unclean.
    /// What an unclean ring held is also kept in a file beside it until `release_held`, and a
    /// start that finds that file returns it instead, so a restart does not lose a fall that is
    /// not settled yet.
    pub fn open(path: &Path) -> Result<(BlackboxRing, RingContents), CollectError> {
        // SAFETY: opens or creates the ring file; the handle is closed on drop.
        let file = unsafe {
            CreateFileW(
                &HSTRING::from(path.as_os_str()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ,
                None,
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?
        };
        // SAFETY: maps RING_BYTES of the file just opened, growing a new file with zeros.
        let mapping =
            unsafe { CreateFileMappingW(file, None, PAGE_READWRITE, 0, RING_BYTES as u32, None) }
                .inspect_err(|_| {
                // SAFETY: closes the file opened above, which nothing else holds.
                let _ = unsafe { CloseHandle(file) };
            })?;
        // SAFETY: a writable view of the whole mapping, unmapped on drop.
        let view = unsafe { MapViewOfFile(mapping, FILE_MAP_WRITE, 0, 0, RING_BYTES) };
        let mut ring = BlackboxRing {
            file,
            mapping,
            view,
            next: 0,
            pending: path.with_extension("pending"),
        };
        if view.Value.is_null() {
            return Err(windows::core::Error::from_thread().into());
        }
        let held = ring.held(wall_ms());
        let bytes = ring.bytes_mut();
        bytes.fill(0);
        bytes[..4].copy_from_slice(&MAGIC);
        bytes[4] = VERSION;
        bytes[5] = WATCHING;
        ring.flush()?;
        Ok((ring, held))
    }

    /// What the previous run left: the bytes of a stop still unsettled, else the ring's own.
    fn held(&self, now_ms: i64) -> RingContents {
        if let Ok(kept) = fs::read(&self.pending)
            && kept.len() == RING_BYTES
        {
            return read(&kept, now_ms);
        }
        let own = read(self.bytes(), now_ms);
        if !own.clean && (!own.samples.is_empty() || !own.tops.is_empty()) {
            let _ = fs::write(&self.pending, self.bytes());
        }
        own
    }

    /// Forgets the stop kept beside the ring: it is settled, or the history it belongs to is gone.
    pub fn release_held(&self) {
        let _ = fs::remove_file(&self.pending);
    }

    pub fn push_sample(&mut self, sample: &Sample) {
        self.put(encode_sample(sample));
    }

    /// Keeps the busiest processes of a ranking: by CPU, by memory and by disk, each once.
    pub fn push_top(&mut self, top: &ProcessTop) {
        for entry in kept(&top.entries) {
            self.put(encode_process(top.ts_ms, entry));
        }
    }

    /// Empties the records, so a fall after this holds nothing from before it.
    pub fn clear(&mut self) {
        self.bytes_mut()[HEADER..].fill(0);
        self.next = 0;
    }

    /// Writes the records to disk.
    pub fn flush(&self) -> Result<(), CollectError> {
        // SAFETY: flushes the dirty pages of the live view, which stays mapped until drop.
        unsafe { FlushViewOfFile(self.view.Value as *const c_void, 0)? };
        Ok(())
    }

    /// Marks the exit as clean and writes everything, so the next start finds no fall.
    pub fn close_cleanly(&mut self) -> Result<(), CollectError> {
        self.bytes_mut()[5] = CLOSED;
        self.flush()
    }

    fn put(&mut self, record: [u8; RECORD]) {
        let at = HEADER + self.next * RECORD;
        self.bytes_mut()[at..at + RECORD].copy_from_slice(&record);
        self.next = (self.next + 1) % RING_RECORDS;
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the view maps RING_BYTES and stays mapped while `self` lives.
        unsafe { std::slice::from_raw_parts(self.view.Value.cast::<u8>(), RING_BYTES) }
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: as in `bytes`; `&mut self` makes this the only reference into the view.
        unsafe { std::slice::from_raw_parts_mut(self.view.Value.cast::<u8>(), RING_BYTES) }
    }
}

impl Drop for BlackboxRing {
    fn drop(&mut self) {
        // SAFETY: unmaps the view and closes both handles opened in `open`, once each.
        unsafe {
            if !self.view.Value.is_null() {
                let _ = UnmapViewOfFile(self.view);
            }
            let _ = CloseHandle(self.mapping);
            let _ = CloseHandle(self.file);
        }
    }
}

/// The samples and rankings in a ring's bytes, oldest first, leaving out records whose time is
/// not plausible at `now_ms`. A ring of another version is an unclean stop with a warning; a file
/// with no ring header counts as clean.
fn read(bytes: &[u8], now_ms: i64) -> RingContents {
    if bytes[..4] != MAGIC {
        return RingContents {
            clean: true,
            ..RingContents::default()
        };
    }
    if bytes[4] != VERSION {
        return RingContents {
            warning: Some(format!(
                "the black box ring has layout version {}, this build reads version {VERSION}",
                bytes[4]
            )),
            ..RingContents::default()
        };
    }
    let plausible = EARLIEST_MS..=now_ms.saturating_add(LATEST_AHEAD_MS);
    let mut samples = Vec::new();
    let mut processes: Vec<(i64, ProcessSample)> = Vec::new();
    for record in bytes[HEADER..HEADER + RING_RECORDS * RECORD]
        .as_chunks::<RECORD>()
        .0
    {
        let ts_ms = i64::from_le_bytes(record[..8].try_into().unwrap_or_default());
        match record[8] {
            _ if !plausible.contains(&ts_ms) => {}
            KIND_SAMPLE => samples.push(decode_sample(ts_ms, record)),
            KIND_PROCESS => processes.push((ts_ms, decode_process(record))),
            _ => {}
        }
    }
    samples.sort_by_key(|s| s.ts_ms);
    processes.sort_by_key(|(ts, _)| *ts);
    let mut tops: Vec<ProcessTop> = Vec::new();
    for (ts_ms, entry) in processes {
        match tops.last_mut() {
            Some(top) if top.ts_ms == ts_ms => top.entries.push(entry),
            _ => tops.push(ProcessTop {
                ts_ms,
                entries: vec![entry],
            }),
        }
    }
    RingContents {
        clean: bytes[5] == CLOSED,
        samples,
        tops,
        warning: None,
    }
}

fn kept(entries: &[ProcessSample]) -> Vec<&ProcessSample> {
    let io = |p: &ProcessSample| p.read_bps + p.write_bps;
    let picks = [
        entries
            .iter()
            .max_by(|a, b| a.cpu_pct.total_cmp(&b.cpu_pct)),
        entries.iter().max_by_key(|p| p.private_bytes),
        entries.iter().max_by(|a, b| io(a).total_cmp(&io(b))),
    ];
    let mut kept: Vec<&ProcessSample> = Vec::with_capacity(TOP_KEPT);
    for pick in picks.into_iter().flatten() {
        if !kept.iter().any(|k| k.pid == pick.pid) {
            kept.push(pick);
        }
    }
    kept
}

/// One second in 64 bytes: the readings, and the disk that waited longest.
fn encode_sample(s: &Sample) -> [u8; RECORD] {
    let mut r = [0u8; RECORD];
    r[..8].copy_from_slice(&s.ts_ms.to_le_bytes());
    r[8] = KIND_SAMPLE;
    put_u16(&mut r, 10, hundredths(s.cpu_pct));
    put_u16(&mut r, 12, hundredths(s.dpc_pct));
    put_u16(&mut r, 14, hundredths(s.isr_pct));
    put_u16(&mut r, 16, s.perf_limit_pct.map_or(u16::MAX, tenths));
    put_u16(&mut r, 18, s.proc_perf_pct.map_or(u16::MAX, tenths));
    put_u16(&mut r, 20, s.gpu_pct.map_or(u16::MAX, hundredths));
    put_u32(&mut r, 24, mib(s.mem_total_bytes));
    put_u32(&mut r, 28, mib(s.mem_avail_bytes));
    put_u32(&mut r, 32, mib(s.commit_bytes));
    put_u32(&mut r, 36, mib(s.commit_limit_bytes));
    r[40..44].copy_from_slice(&s.hard_faults_per_s.unwrap_or(f32::NAN).to_le_bytes());
    r[44..48].copy_from_slice(&s.probe_max_lag_ms.to_le_bytes());
    r[48..52].copy_from_slice(&s.sampler_lag_ms.to_le_bytes());
    let worst = s.disks.iter().max_by(|a, b| {
        let rank = |d: &DiskLoad| (d.latency_ms.unwrap_or(f32::NEG_INFINITY), d.active_pct);
        rank(a)
            .partial_cmp(&rank(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if let Some(disk) = worst {
        put_u16(&mut r, 22, hundredths(disk.active_pct));
        r[52..56].copy_from_slice(&disk.latency_ms.unwrap_or(f32::NAN).to_le_bytes());
        put_str(&mut r[56..64], &disk.name);
    }
    r
}

fn decode_sample(ts_ms: i64, r: &[u8]) -> Sample {
    let name = get_str(&r[56..64]);
    let gpu = get_u16(r, 20);
    let tenths = |at: usize| Some(get_u16(r, at)).filter(|v| *v != u16::MAX);
    let known = |at: usize| Some(get_f32(r, at)).filter(|v| !v.is_nan());
    Sample {
        ts_ms,
        cpu_pct: f32::from(get_u16(r, 10)) / 100.0,
        dpc_pct: f32::from(get_u16(r, 12)) / 100.0,
        isr_pct: f32::from(get_u16(r, 14)) / 100.0,
        perf_limit_pct: tenths(16).map(|v| f32::from(v) / 10.0),
        proc_perf_pct: tenths(18).map(|v| f32::from(v) / 10.0),
        mem_total_bytes: u64::from(get_u32(r, 24)) * MIB,
        mem_avail_bytes: u64::from(get_u32(r, 28)) * MIB,
        commit_bytes: u64::from(get_u32(r, 32)) * MIB,
        commit_limit_bytes: u64::from(get_u32(r, 36)) * MIB,
        hard_faults_per_s: known(40),
        gpu_pct: (gpu != u16::MAX).then(|| f32::from(gpu) / 100.0),
        probe_max_lag_ms: get_f32(r, 44),
        sampler_lag_ms: get_f32(r, 48),
        cores: Vec::new(),
        disks: if name.is_empty() {
            Vec::new()
        } else {
            vec![DiskLoad {
                name,
                active_pct: f32::from(get_u16(r, 22)) / 100.0,
                latency_ms: known(52),
                queue_len: None,
                bytes_per_s: None,
            }]
        },
    }
}

/// One process of a ranking in 64 bytes, its name cut to 32 bytes.
fn encode_process(ts_ms: i64, p: &ProcessSample) -> [u8; RECORD] {
    let mut r = [0u8; RECORD];
    r[..8].copy_from_slice(&ts_ms.to_le_bytes());
    r[8] = KIND_PROCESS;
    put_u16(&mut r, 10, hundredths(p.cpu_pct));
    put_u16(&mut r, 12, hundredths(p.gpu_pct));
    put_u32(&mut r, 16, p.pid);
    put_u32(&mut r, 20, mib(p.private_bytes));
    put_u32(
        &mut r,
        24,
        (p.read_bps / 1024.0).clamp(0.0, u32::MAX as f32) as u32,
    );
    put_u32(
        &mut r,
        28,
        (p.write_bps / 1024.0).clamp(0.0, u32::MAX as f32) as u32,
    );
    put_str(&mut r[32..64], &p.name);
    r
}

fn decode_process(r: &[u8]) -> ProcessSample {
    ProcessSample {
        pid: get_u32(r, 16),
        name: get_str(&r[32..64]),
        cpu_pct: f32::from(get_u16(r, 10)) / 100.0,
        private_bytes: u64::from(get_u32(r, 20)) * MIB,
        read_bps: get_u32(r, 24) as f32 * 1024.0,
        write_bps: get_u32(r, 28) as f32 * 1024.0,
        gpu_pct: f32::from(get_u16(r, 12)) / 100.0,
    }
}

fn hundredths(v: f32) -> u16 {
    (v * 100.0).round().clamp(0.0, f32::from(u16::MAX - 1)) as u16
}

fn tenths(v: f32) -> u16 {
    (v * 10.0).round().clamp(0.0, f32::from(u16::MAX - 1)) as u16
}

fn mib(bytes: u64) -> u32 {
    u32::try_from(bytes / MIB).unwrap_or(u32::MAX)
}

fn put_u16(r: &mut [u8], at: usize, v: u16) {
    r[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32(r: &mut [u8], at: usize, v: u32) {
    r[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn get_u16(r: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([r[at], r[at + 1]])
}

fn get_u32(r: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([r[at], r[at + 1], r[at + 2], r[at + 3]])
}

fn get_f32(r: &[u8], at: usize) -> f32 {
    f32::from_bits(get_u32(r, at))
}

/// Copies `s` into `field`, cut at a character boundary, the rest zero.
fn put_str(field: &mut [u8], s: &str) {
    let mut end = s.len().min(field.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    field[..end].copy_from_slice(&s.as_bytes()[..end]);
}

fn get_str(field: &[u8]) -> String {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ts_ms: i64) -> Sample {
        Sample {
            ts_ms,
            cpu_pct: 37.25,
            dpc_pct: 1.5,
            isr_pct: 0.25,
            perf_limit_pct: Some(100.0),
            proc_perf_pct: Some(112.4),
            mem_total_bytes: 32 * 1024 * MIB,
            mem_avail_bytes: 1200 * MIB,
            commit_bytes: 40 * 1024 * MIB,
            commit_limit_bytes: 44 * 1024 * MIB,
            hard_faults_per_s: Some(1840.0),
            gpu_pct: Some(88.5),
            probe_max_lag_ms: 212.5,
            sampler_lag_ms: 3.0,
            cores: Vec::new(),
            disks: vec![
                DiskLoad {
                    name: "0 C:".into(),
                    active_pct: 12.0,
                    latency_ms: Some(2.0),
                    queue_len: Some(0.1),
                    bytes_per_s: Some(1e6),
                },
                DiskLoad {
                    name: "1 D:".into(),
                    active_pct: 99.0,
                    latency_ms: Some(840.0),
                    queue_len: Some(6.0),
                    bytes_per_s: Some(4e6),
                },
            ],
        }
    }

    fn process(pid: u32, name: &str, cpu: f32, mib_used: u64, io: f32) -> ProcessSample {
        ProcessSample {
            pid,
            name: name.into(),
            cpu_pct: cpu,
            private_bytes: mib_used * MIB,
            read_bps: io,
            write_bps: 0.0,
            gpu_pct: 0.0,
        }
    }

    #[test]
    fn a_sample_keeps_its_readings_and_the_slowest_disk() {
        let back = decode_sample(5_000, &encode_sample(&sample(5_000)));
        assert_eq!(back.cpu_pct, 37.25);
        assert_eq!(back.proc_perf_pct, Some(112.4));
        assert_eq!(back.mem_avail_bytes, 1200 * MIB);
        assert_eq!(back.gpu_pct, Some(88.5));
        assert_eq!(back.probe_max_lag_ms, 212.5);
        assert_eq!(back.disks.len(), 1);
        assert_eq!(back.disks[0].name, "1 D:");
        assert_eq!(back.disks[0].latency_ms, Some(840.0));
        assert_eq!(back.disks[0].active_pct, 99.0);
    }

    #[test]
    fn a_sample_without_its_counters_decodes_them_as_absent() {
        let mut bare = sample(7_000);
        bare.perf_limit_pct = None;
        bare.proc_perf_pct = None;
        bare.hard_faults_per_s = None;
        bare.disks[1].latency_ms = None;

        let back = decode_sample(7_000, &encode_sample(&bare));

        assert_eq!(back.perf_limit_pct, None);
        assert_eq!(back.proc_perf_pct, None);
        assert_eq!(back.hard_faults_per_s, None);
        assert_eq!(back.cpu_pct, 37.25);
        // The disk that waited longest is the one with a latency; the other has none to rank.
        assert_eq!(back.disks[0].name, "0 C:");
        assert_eq!(back.disks[0].latency_ms, Some(2.0));
    }

    #[test]
    fn a_disk_without_a_latency_keeps_its_place_with_none() {
        let mut bare = sample(8_000);
        bare.disks.remove(0);
        bare.disks[0].latency_ms = None;

        let back = decode_sample(8_000, &encode_sample(&bare));

        assert_eq!(back.disks[0].name, "1 D:");
        assert_eq!(back.disks[0].latency_ms, None);
        assert_eq!(back.disks[0].active_pct, 99.0);
    }

    #[test]
    fn a_ranking_keeps_the_busiest_by_cpu_memory_and_disk_once_each() {
        let entries = vec![
            process(1, "Game.exe", 60.0, 9000, 1e5),
            process(2, "Browser.exe", 5.0, 14000, 2e5),
            process(3, "Indexer.exe", 1.0, 100, 9e7),
            process(4, "Idle.exe", 0.1, 10, 0.0),
        ];
        let names: Vec<&str> = kept(&entries).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Game.exe", "Browser.exe", "Indexer.exe"]);
        let one = vec![process(1, "Game.exe", 60.0, 9000, 9e7)];
        assert_eq!(kept(&one).len(), 1);
    }

    #[test]
    fn a_long_name_is_cut_at_a_character_boundary() {
        let name = "ProgramaComNomeMuitoLongoDemaisÇÇÇ.exe";
        let back = decode_process(&encode_process(1, &process(9, name, 1.0, 1, 0.0)));
        assert!(name.starts_with(&back.name));
        assert!(back.name.len() <= 32);
    }

    /// A clock reading after the earliest plausible time, for rings written in tests.
    const NOW: i64 = 1_800_000_000_000;

    fn ring_with(records: &[[u8; RECORD]]) -> Vec<u8> {
        let mut bytes = vec![0u8; RING_BYTES];
        bytes[..4].copy_from_slice(&MAGIC);
        bytes[4] = VERSION;
        for (i, record) in records.iter().enumerate() {
            bytes[HEADER + i * RECORD..HEADER + (i + 1) * RECORD].copy_from_slice(record);
        }
        bytes
    }

    #[test]
    fn reading_orders_seconds_and_groups_rankings() {
        let bytes = ring_with(&[
            encode_sample(&sample(NOW - 2_000)),
            encode_process(NOW - 3_000, &process(1, "A.exe", 1.0, 1, 0.0)),
            encode_sample(&sample(NOW - 4_000)),
            encode_process(NOW - 3_000, &process(2, "B.exe", 1.0, 1, 0.0)),
        ]);
        let held = read(&bytes, NOW);
        assert!(!held.clean);
        assert_eq!(
            held.samples.iter().map(|s| s.ts_ms).collect::<Vec<_>>(),
            [NOW - 4_000, NOW - 2_000]
        );
        assert_eq!(held.tops.len(), 1);
        assert_eq!(held.tops[0].entries.len(), 2);
    }

    #[test]
    fn an_unknown_layout_counts_as_clean() {
        assert!(read(&vec![0u8; RING_BYTES], NOW).clean);
    }

    #[test]
    fn a_record_with_an_implausible_time_is_dropped() {
        let bytes = ring_with(&[
            encode_sample(&sample(i64::MAX)),
            encode_sample(&sample(1)),
            encode_sample(&sample(-NOW)),
            encode_sample(&sample(NOW + 2 * LATEST_AHEAD_MS)),
            encode_process(1, &process(1, "A.exe", 1.0, 1, 0.0)),
            encode_sample(&sample(NOW - 1_000)),
            encode_process(NOW - 1_000, &process(2, "B.exe", 1.0, 1, 0.0)),
        ]);
        let held = read(&bytes, NOW);
        assert_eq!(
            held.samples.iter().map(|s| s.ts_ms).collect::<Vec<_>>(),
            [NOW - 1_000]
        );
        assert_eq!(held.tops.len(), 1);
        assert_eq!(held.tops[0].entries[0].name, "B.exe");
    }

    #[test]
    fn a_ring_of_another_version_is_a_pending_fall() {
        let mut bytes = ring_with(&[encode_sample(&sample(NOW - 1_000))]);
        bytes[4] = VERSION + 1;
        let held = read(&bytes, NOW);
        assert!(!held.clean);
        assert!(held.samples.is_empty());
        assert!(held.warning.is_some_and(|w| w.contains("version")));
    }

    #[test]
    fn a_readable_ring_has_no_warning() {
        let held = read(&ring_with(&[encode_sample(&sample(NOW - 1_000))]), NOW);
        assert_eq!(held.warning, None);
    }
}
