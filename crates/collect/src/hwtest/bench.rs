use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use vestigium_core::Media;
use vestigium_core::health::Subject;
use vestigium_core::hwtest::{
    BenchMode, BenchResult, Benchmark, Bus, Outcome, TestData, TestRun, bench_has_room,
};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE,
};
use windows::Win32::Storage::FileSystem::{
    CREATE_NEW, CreateFileW, FILE_FLAG_DELETE_ON_CLOSE, FILE_FLAG_NO_BUFFERING,
    FILE_FLAG_OVERLAPPED, FILE_FLAG_WRITE_THROUGH, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, GetDiskFreeSpaceExW, OPEN_EXISTING, ReadFile, WriteFile,
};
use windows::Win32::System::IO::{
    CancelIoEx, CreateIoCompletionPort, DeviceIoControl, GetQueuedCompletionStatus, OVERLAPPED,
};
use windows::Win32::System::Ioctl::{DISK_PERFORMANCE, IOCTL_DISK_PERFORMANCE};
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc, VirtualFree,
};
use windows::core::HSTRING;

use crate::{monotonic_ms, wall_ms};

/// Room for the deepest queue of the largest blocks: 8 × 1 MiB, rounded up.
const BUFFERS: usize = 32 << 20;
/// Longest wait for one I/O before the run gives up.
const IO_TIMEOUT_MS: u32 = 30_000;

/// The volume a benchmark measures and the disk under it.
#[derive(Clone, Debug)]
pub struct BenchTarget {
    /// A folder on the volume where a new file may be created without elevation.
    pub dir: PathBuf,
    /// Root of the volume, such as "C:\".
    pub volume: String,
    /// The physical drive under the volume, to tell whether other programs used it meanwhile.
    pub disk: Option<u32>,
    pub bus: Bus,
    pub media: Media,
}

/// A handle closed when dropped.
struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle was opened here and is closed once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Page-aligned I/O buffers, freed when dropped.
struct Buffers(*mut u8);

impl Buffers {
    fn new() -> Option<Buffers> {
        // SAFETY: a fresh read-write region released in `Drop`.
        let base = unsafe { VirtualAlloc(None, BUFFERS, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };
        if base.is_null() {
            return None;
        }
        // SAFETY: the region was just allocated with BUFFERS bytes.
        let words = unsafe { std::slice::from_raw_parts_mut(base.cast::<u64>(), BUFFERS / 8) };
        let mut state = 0x2545_f491_4f6c_dd1du64;
        for word in words {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *word = state;
        }
        Some(Buffers(base.cast()))
    }
}

impl Drop for Buffers {
    fn drop(&mut self) {
        // SAFETY: releases the region allocated in `new`, once.
        let _ = unsafe { VirtualFree(self.0.cast(), 0, MEM_RELEASE) };
    }
}

#[repr(C)]
struct Slot {
    /// First, so a completed OVERLAPPED pointer is the slot's own.
    ov: OVERLAPPED,
    buf: *mut u8,
    issued_ms: f64,
}

struct Measured {
    bytes: u64,
    ios: u64,
    secs: f64,
    max_ms: f64,
}

/// Bytes the disk read and wrote since it started, for every program together.
fn disk_bytes(disk: u32) -> Option<u64> {
    let path = HSTRING::from(format!(r"\\.\PhysicalDrive{disk}"));
    // SAFETY: opens the device for a query only; `Owned` closes it.
    let drive = Owned(
        unsafe {
            CreateFileW(
                &path,
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        }
        .ok()?,
    );
    let mut perf = DISK_PERFORMANCE::default();
    let mut returned = 0u32;
    // SAFETY: the output is a live DISK_PERFORMANCE of the size passed.
    unsafe {
        DeviceIoControl(
            drive.0,
            IOCTL_DISK_PERFORMANCE,
            None,
            0,
            Some((&raw mut perf).cast::<c_void>()),
            size_of::<DISK_PERFORMANCE>() as u32,
            Some(&mut returned),
            None,
        )
    }
    .ok()?;
    Some((perf.BytesRead + perf.BytesWritten) as u64)
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Keeps `depth` I/Os of `block` bytes in flight on `file` until `seconds` pass, `cap` bytes were
/// issued or `stop` is set; a sequential write ends after one pass over the file.
#[allow(clippy::too_many_arguments)]
fn run_io(
    file: HANDLE,
    port: HANDLE,
    buffers: &Buffers,
    file_bytes: u64,
    block: u32,
    depth: u32,
    sequential: bool,
    write: bool,
    seconds: f64,
    cap: u64,
    stop: &AtomicBool,
) -> Result<Measured, String> {
    let block_bytes = u64::from(block);
    let blocks = (file_bytes / block_bytes).max(1);
    let mut rng = 0x1234_5678_9abc_def1u64 ^ monotonic_ms().to_bits();
    let mut next_seq = 0u64;
    let mut slots: Vec<Box<Slot>> = (0..depth as usize)
        .map(|i| {
            Box::new(Slot {
                ov: OVERLAPPED::default(),
                // SAFETY: each slot gets its own block inside the BUFFERS region.
                buf: unsafe { buffers.0.add(i * block as usize) },
                issued_ms: 0.0,
            })
        })
        .collect();
    let started = monotonic_ms();
    let (mut bytes, mut ios, mut issued, mut max_ms) = (0u64, 0u64, 0u64, 0f64);
    let mut in_flight = 0usize;
    let mut failure = None;

    let mut issue = |slot: &mut Slot, issued: &mut u64| -> Result<bool, String> {
        if stop.load(Ordering::Relaxed)
            || (monotonic_ms() - started) / 1000.0 >= seconds
            || *issued >= cap
        {
            return Ok(false);
        }
        let index = if sequential {
            if next_seq >= blocks {
                if write {
                    return Ok(false);
                }
                next_seq = 0;
            }
            next_seq += 1;
            next_seq - 1
        } else {
            next_random(&mut rng) % blocks
        };
        let offset = index * block_bytes;
        slot.ov = OVERLAPPED::default();
        slot.ov.Anonymous.Anonymous.Offset = offset as u32;
        slot.ov.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
        slot.issued_ms = monotonic_ms();
        // SAFETY: the slot's buffer holds `block` bytes and outlives the I/O, which completes on
        // the port before the slot is reused or dropped.
        let result = unsafe {
            let buf = std::slice::from_raw_parts_mut(slot.buf, block as usize);
            if write {
                WriteFile(file, Some(buf), None, Some(&mut slot.ov))
            } else {
                ReadFile(file, Some(buf), None, Some(&mut slot.ov))
            }
        };
        match result {
            Ok(()) => {}
            Err(e) if e.code() == ERROR_IO_PENDING.to_hresult() => {}
            Err(e) => return Err(e.to_string()),
        }
        *issued += block_bytes;
        Ok(true)
    };

    for slot in slots.iter_mut() {
        match issue(slot, &mut issued) {
            Ok(true) => in_flight += 1,
            Ok(false) => {}
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }
    while in_flight > 0 {
        let (mut done, mut key) = (0u32, 0usize);
        let mut ov: *mut OVERLAPPED = std::ptr::null_mut();
        // SAFETY: waits for one completion on the port this run owns.
        let waited =
            unsafe { GetQueuedCompletionStatus(port, &mut done, &mut key, &mut ov, IO_TIMEOUT_MS) };
        if ov.is_null() {
            failure.get_or_insert_with(|| "the disk stopped answering".into());
            // SAFETY: cancels this run's own pending I/O on the file before the slots go away.
            let _ = unsafe { CancelIoEx(file, None) };
            break;
        }
        in_flight -= 1;
        // SAFETY: every OVERLAPPED issued is the first field of a boxed `Slot` in `slots`.
        let slot = unsafe { &mut *ov.cast::<Slot>() };
        if let Err(e) = waited {
            failure.get_or_insert(e.to_string());
            continue;
        }
        max_ms = max_ms.max(monotonic_ms() - slot.issued_ms);
        bytes += u64::from(done);
        ios += 1;
        if failure.is_none() {
            match issue(slot, &mut issued) {
                Ok(true) => in_flight += 1,
                Ok(false) => {}
                Err(e) => failure = Some(e),
            }
        }
    }
    match failure {
        Some(e) => Err(e),
        None => Ok(Measured {
            bytes,
            ios,
            secs: ((monotonic_ms() - started) / 1000.0).max(1e-3),
            max_ms,
        }),
    }
}

/// Bytes free to this user on the volume that holds `dir`, when it can be asked.
fn free_bytes(dir: &Path) -> Option<u64> {
    let mut free = 0u64;
    // SAFETY: the output outlives the call and the path is NUL-terminated by `HSTRING`.
    unsafe { GetDiskFreeSpaceExW(&HSTRING::from(dir.as_os_str()), Some(&mut free), None, None) }
        .ok()?;
    Some(free)
}

fn open_file(path: &Path) -> Result<Owned, String> {
    // SAFETY: a new file this run owns, deleted when its handle closes.
    let file = unsafe {
        CreateFileW(
            &HSTRING::from(path.as_os_str()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_MODE(0),
            None,
            CREATE_NEW,
            FILE_FLAG_NO_BUFFERING
                | FILE_FLAG_WRITE_THROUGH
                | FILE_FLAG_OVERLAPPED
                | FILE_FLAG_DELETE_ON_CLOSE,
            None,
        )
    };
    file.map(Owned).map_err(|e| e.to_string())
}

/// Measures the volume with a temporary file of `file_bytes`, written once before measuring;
/// every mode runs at normal priority for at most `seconds` (`BENCH_SECONDS` on the screen).
pub fn benchmark(
    target: &BenchTarget,
    file_bytes: u64,
    seconds: f64,
    stop: &AtomicBool,
    progress: &mut dyn FnMut(BenchMode),
) -> TestRun {
    let started_ms = wall_ms();
    let mut bench = Benchmark {
        volume: target.volume.clone(),
        bus: target.bus,
        media: target.media,
        file_bytes,
        written_bytes: 0,
        results: Vec::new(),
        outside_io: false,
        failure: None,
    };
    let disk_before = target.disk.and_then(disk_bytes);
    let mut ours = 0u64;
    let path = target
        .dir
        .join(format!("vestigium-bench-{}.tmp", std::process::id()));
    let measured = (|| -> Result<(), String> {
        if free_bytes(&target.dir).is_some_and(|free| !bench_has_room(free, file_bytes)) {
            return Err(format!(
                "not enough free space on {} for a file of {file_bytes} bytes",
                target.volume
            ));
        }
        let file = open_file(&path)?;
        // SAFETY: binds the file to a new completion port, closed by `Owned`.
        let port = Owned(
            unsafe { CreateIoCompletionPort(file.0, None, 0, 1) }.map_err(|e| e.to_string())?,
        );
        let buffers = Buffers::new().ok_or("no memory for the buffers")?;
        let prep = run_io(
            file.0,
            port.0,
            &buffers,
            file_bytes,
            1 << 20,
            8,
            true,
            true,
            f64::MAX,
            file_bytes,
            stop,
        )?;
        bench.written_bytes += prep.bytes;
        ours += prep.bytes;
        for mode in BenchMode::ALL {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            progress(mode);
            let run = run_io(
                file.0,
                port.0,
                &buffers,
                file_bytes,
                mode.block_bytes(),
                mode.depth(),
                matches!(mode, BenchMode::SeqRead | BenchMode::SeqWrite),
                mode.writes(),
                seconds,
                mode.write_cap(file_bytes),
                stop,
            )?;
            if mode.writes() {
                bench.written_bytes += run.bytes;
            }
            ours += run.bytes;
            bench.results.push(BenchResult {
                mode,
                mb_per_s: (run.bytes as f64 / f64::from(1 << 20) / run.secs) as f32,
                iops: (run.ios as f64 / run.secs) as f32,
                max_ms: run.max_ms as f32,
            });
        }
        Ok(())
    })();
    if let (Some(before), Some(after)) = (disk_before, target.disk.and_then(disk_bytes)) {
        let all = after.saturating_sub(before);
        bench.outside_io = all > ours + (ours / 10).max(16 << 20);
    }
    let outcome = match measured {
        Err(e) => {
            bench.failure = Some(e);
            Outcome::Failed
        }
        Ok(()) if stop.load(Ordering::Relaxed) => Outcome::Interrupted,
        Ok(()) => Outcome::Clean,
    };
    TestRun {
        subject: Subject::Volume(target.volume.clone()),
        started_ms,
        ended_ms: wall_ms(),
        outcome,
        data: TestData::DiskBenchmark(bench),
    }
}
