use serde::{Deserialize, Serialize};

use crate::Media;

/// The surface scan reads the disk in blocks of this many bytes.
pub const SURFACE_BLOCK: u32 = 1 << 20;
/// The disk map has this many regions, each holding its slowest block.
pub const SURFACE_REGIONS: usize = 1024;
/// Upper bounds of the latency bands, in ms; the last band holds everything at or above 500 ms.
pub const BAND_BOUNDS_MS: [f32; 5] = [5.0, 20.0, 50.0, 150.0, 500.0];
pub const BANDS: usize = BAND_BOUNDS_MS.len() + 1;
/// A block this slow is read again, and counts as slow only when it is slow twice.
pub const SLOW_BLOCK_MS: f32 = 500.0;
/// Slowest blocks kept with their position.
pub const WORST_KEPT: usize = 100;
/// Unreadable sectors kept with their position; the count goes on past them.
pub const UNREADABLE_KEPT: usize = 1000;

/// The band of a block read in `ms`.
pub fn latency_band(ms: f32) -> usize {
    BAND_BOUNDS_MS
        .iter()
        .position(|&bound| ms < bound)
        .unwrap_or(BANDS - 1)
}

/// One region of the disk map.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Region {
    Unread,
    Read {
        worst_ms: f32,
    },
    /// Holds at least one sector that could not be read.
    Unreadable,
}

impl Region {
    fn merge(self, other: Region) -> Region {
        match (self, other) {
            (Region::Unreadable, _) | (_, Region::Unreadable) => Region::Unreadable,
            (Region::Read { worst_ms: a }, Region::Read { worst_ms: b }) => {
                Region::Read { worst_ms: a.max(b) }
            }
            (Region::Unread, region) | (region, Region::Unread) => region,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SlowBlock {
    pub offset: u64,
    pub ms: f32,
}

/// What a read-only surface scan of one physical disk covered and found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceScan {
    pub disk: u32,
    pub media: Media,
    pub disk_bytes: u64,
    pub sector_bytes: u32,
    /// The stretch read, in bytes from the start of the disk.
    pub from_bytes: u64,
    pub to_bytes: u64,
    /// Blocks per latency band.
    pub bands: [u64; BANDS],
    pub unreadable_blocks: u64,
    pub regions: Vec<Region>,
    /// The slowest blocks, slowest first.
    pub worst: Vec<SlowBlock>,
    /// Byte offsets of the sectors that could not be read.
    pub unreadable: Vec<u64>,
    /// Why the scan could not run or stopped short, when it did.
    pub failure: Option<String>,
}

impl SurfaceScan {
    /// Read from the first byte to the last.
    pub fn whole_disk(&self) -> bool {
        self.from_bytes == 0 && self.to_bytes >= self.disk_bytes
    }

    pub fn slow_blocks(&self) -> u64 {
        self.bands[BANDS - 1]
    }
}

/// Where a running scan stands, rewritten every few seconds for the window.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceProgress {
    pub disk: u32,
    pub ts_ms: i64,
    pub at_bytes: u64,
    pub from_bytes: u64,
    pub disk_bytes: u64,
    pub bands: [u64; BANDS],
    pub unreadable_blocks: u64,
}

/// Builds a `SurfaceScan` from the blocks the scan reads, in order.
#[derive(Clone, Debug)]
pub struct SurfaceAggregator {
    scan: SurfaceScan,
}

impl SurfaceAggregator {
    pub fn new(
        disk: u32,
        media: Media,
        disk_bytes: u64,
        sector_bytes: u32,
        from_bytes: u64,
    ) -> SurfaceAggregator {
        SurfaceAggregator {
            scan: SurfaceScan {
                disk,
                media,
                disk_bytes,
                sector_bytes,
                from_bytes,
                to_bytes: from_bytes,
                bands: [0; BANDS],
                unreadable_blocks: 0,
                regions: vec![Region::Unread; SURFACE_REGIONS],
                worst: Vec::new(),
                unreadable: Vec::new(),
                failure: None,
            },
        }
    }

    fn region(&self, offset: u64) -> usize {
        let disk = u128::from(self.scan.disk_bytes.max(1));
        ((u128::from(offset) * SURFACE_REGIONS as u128 / disk) as usize).min(SURFACE_REGIONS - 1)
    }

    /// A block at `offset` read in `ms`.
    pub fn block(&mut self, offset: u64, ms: f32) {
        self.scan.bands[latency_band(ms)] += 1;
        let region = self.region(offset);
        self.scan.regions[region] = self.scan.regions[region].merge(Region::Read { worst_ms: ms });
        let at = self.scan.worst.partition_point(|b| b.ms >= ms);
        if at < WORST_KEPT {
            self.scan.worst.insert(at, SlowBlock { offset, ms });
            self.scan.worst.truncate(WORST_KEPT);
        }
    }

    /// A block at `offset` with sectors that could not be read, by their byte offsets.
    pub fn unreadable(&mut self, offset: u64, sectors: &[u64]) {
        self.scan.unreadable_blocks += 1;
        let region = self.region(offset);
        self.scan.regions[region] = Region::Unreadable;
        let room = UNREADABLE_KEPT.saturating_sub(self.scan.unreadable.len());
        self.scan
            .unreadable
            .extend(sectors.iter().take(room).copied());
    }

    pub fn progress(&self, ts_ms: i64, at_bytes: u64) -> SurfaceProgress {
        SurfaceProgress {
            disk: self.scan.disk,
            ts_ms,
            at_bytes,
            from_bytes: self.scan.from_bytes,
            disk_bytes: self.scan.disk_bytes,
            bands: self.scan.bands,
            unreadable_blocks: self.scan.unreadable_blocks,
        }
    }

    pub fn finish(mut self, to_bytes: u64, failure: Option<String>) -> SurfaceScan {
        self.scan.to_bytes = to_bytes.min(self.scan.disk_bytes);
        self.scan.failure = failure;
        self.scan
    }
}

/// One view of a disk from its scans, oldest first: the newest, extended back by every earlier
/// scan that stopped exactly where the next one started.
pub fn combined_surface(runs: &[SurfaceScan]) -> Option<SurfaceScan> {
    let mut combined = runs.last()?.clone();
    for earlier in runs.iter().rev().skip(1) {
        if earlier.to_bytes != combined.from_bytes || earlier.disk_bytes != combined.disk_bytes {
            break;
        }
        combined.from_bytes = earlier.from_bytes;
        for (band, count) in combined.bands.iter_mut().zip(earlier.bands) {
            *band += count;
        }
        combined.unreadable_blocks += earlier.unreadable_blocks;
        for (region, old) in combined.regions.iter_mut().zip(&earlier.regions) {
            *region = region.merge(*old);
        }
        combined.worst.extend(&earlier.worst);
        combined.worst.sort_by(|a, b| b.ms.total_cmp(&a.ms));
        combined.worst.truncate(WORST_KEPT);
        let room = UNREADABLE_KEPT.saturating_sub(combined.unreadable.len());
        combined
            .unreadable
            .extend(earlier.unreadable.iter().take(room).copied());
    }
    Some(combined)
}

/// Size of the benchmark's temporary file.
pub const BENCH_FILE_BYTES: u64 = 1 << 30;
/// Longest a benchmark mode runs.
pub const BENCH_SECONDS: f64 = 5.0;

/// How a disk is attached, as the storage stack reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bus {
    Sata,
    Nvme,
    Usb,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BenchMode {
    /// 1 MiB blocks, 8 in flight.
    SeqRead,
    SeqWrite,
    /// 4 KiB blocks at random, 32 in flight.
    RandRead32,
    RandWrite32,
    /// 4 KiB blocks at random, one at a time.
    RandRead1,
    RandWrite1,
}

impl BenchMode {
    pub const ALL: [BenchMode; 6] = [
        BenchMode::SeqRead,
        BenchMode::SeqWrite,
        BenchMode::RandRead32,
        BenchMode::RandWrite32,
        BenchMode::RandRead1,
        BenchMode::RandWrite1,
    ];

    pub fn block_bytes(self) -> u32 {
        match self {
            BenchMode::SeqRead | BenchMode::SeqWrite => 1 << 20,
            _ => 4096,
        }
    }

    pub fn depth(self) -> u32 {
        match self {
            BenchMode::SeqRead | BenchMode::SeqWrite => 8,
            BenchMode::RandRead32 | BenchMode::RandWrite32 => 32,
            BenchMode::RandRead1 | BenchMode::RandWrite1 => 1,
        }
    }

    /// Most a mode writes: one pass over the file sequentially, less at random.
    pub fn write_cap(self, file_bytes: u64) -> u64 {
        match self {
            BenchMode::SeqWrite => file_bytes,
            BenchMode::RandWrite32 => file_bytes.min(256 << 20),
            BenchMode::RandWrite1 => file_bytes.min(64 << 20),
            _ => u64::MAX,
        }
    }

    pub fn writes(self) -> bool {
        matches!(
            self,
            BenchMode::SeqWrite | BenchMode::RandWrite32 | BenchMode::RandWrite1
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchResult {
    pub mode: BenchMode,
    pub mb_per_s: f32,
    pub iops: f32,
    pub max_ms: f32,
}

/// What a quick benchmark of one volume measured, on a temporary file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Benchmark {
    /// Root of the volume, such as "C:\".
    pub volume: String,
    pub bus: Bus,
    pub media: Media,
    pub file_bytes: u64,
    /// Everything the run wrote, the preparation included.
    pub written_bytes: u64,
    pub results: Vec<BenchResult>,
    /// Other programs used the disk during the run, so the numbers may come out low.
    pub outside_io: bool,
    pub failure: Option<String>,
}

/// Where a result falls against the range typical of its kind of disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expected {
    Below,
    Inside,
    Above,
}

/// The MB/s range typical of a disk of this kind in this mode; `None` where disks vary too much.
pub fn bench_reference(bus: Bus, media: Media, mode: BenchMode) -> Option<(f32, f32)> {
    use BenchMode::*;
    let range = match (media, bus) {
        (Media::Hdd, _) => match mode {
            SeqRead | SeqWrite => (80.0, 250.0),
            RandRead32 | RandWrite32 => (0.5, 3.0),
            RandRead1 => (0.3, 1.5),
            RandWrite1 => (0.3, 3.0),
        },
        (Media::Ssd, Bus::Sata) => match mode {
            SeqRead => (350.0, 560.0),
            SeqWrite => (250.0, 530.0),
            RandRead32 => (150.0, 400.0),
            RandWrite32 => (150.0, 380.0),
            RandRead1 => (20.0, 60.0),
            RandWrite1 => (40.0, 150.0),
        },
        (Media::Ssd, Bus::Nvme) => match mode {
            SeqRead => (1500.0, 7500.0),
            SeqWrite => (1000.0, 7000.0),
            RandRead32 | RandWrite32 => (300.0, 3000.0),
            RandRead1 => (40.0, 100.0),
            RandWrite1 => (100.0, 300.0),
        },
        _ => return None,
    };
    Some(range)
}

pub fn bench_verdict(result: &BenchResult, bus: Bus, media: Media) -> Option<Expected> {
    let (low, high) = bench_reference(bus, media, result.mode)?;
    Some(if result.mb_per_s < low {
        Expected::Below
    } else if result.mb_per_s > high {
        Expected::Above
    } else {
        Expected::Inside
    })
}

/// What `chkdsk /scan` said about a volume, from its exit code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeResult {
    NoProblems,
    ProblemsFound,
    CouldNotCheck,
}

/// What an online file system check of one volume found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolumeCheck {
    /// Root of the volume, such as "C:\".
    pub volume: String,
    pub exit_code: Option<u32>,
    pub result: VolumeResult,
    /// chkdsk's own text, in the console code page and language, for the technical level.
    pub output: String,
}

/// chkdsk exit codes: 0 clean, 2 only cleanup left, 1 and 3 problems found.
pub fn volume_result(exit_code: u32) -> VolumeResult {
    match exit_code {
        0 | 2 => VolumeResult::NoProblems,
        1 | 3 => VolumeResult::ProblemsFound,
        _ => VolumeResult::CouldNotCheck,
    }
}
