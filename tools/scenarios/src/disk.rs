use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::windows::fs::{FileExt, OpenOptionsExt};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Storage::FileSystem::FILE_FLAG_NO_BUFFERING;

use crate::stopped;

/// Read size and alignment; fits disks with 512-byte and 4 KiB sectors.
const BLOCK: usize = 4096;
const WRITE_CHUNK: usize = 1 << 20;
const GIB: u64 = 1 << 30;
/// On and off periods of `io-burst`.
pub const BURST: (Duration, Duration) = (Duration::from_secs(2), Duration::from_secs(3));

/// One read buffer, aligned as uncached I/O requires.
#[repr(C, align(4096))]
struct Block([u8; BLOCK]);

/// Size of the file to read; creates it once when `create_gb` asks for more than exists.
pub fn prepare(file: &Path, create_gb: Option<u64>) -> Result<u64, String> {
    let want = create_gb.map(|gb| gb.clamp(1, 64) * GIB);
    let have = std::fs::metadata(file).map(|m| m.len()).ok();
    match (have, want) {
        (Some(len), None) => Ok(len),
        (Some(len), Some(want)) if len >= want => Ok(len),
        (_, Some(want)) => create(file, want).map(|()| want),
        (None, None) => Err(format!(
            "{} does not exist; pass --create-gb N to create it once",
            file.display()
        )),
    }
}

/// Writes `bytes` of random data, the only time the scenarios write to a disk.
fn create(file: &Path, bytes: u64) -> Result<(), String> {
    println!(
        "Creating {} ({} GiB); later runs only read it.",
        file.display(),
        bytes / GIB
    );
    let mut out = File::create(file).map_err(|e| e.to_string())?;
    let mut chunk = vec![0u8; WRITE_CHUNK];
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut written = 0u64;
    while written < bytes {
        if stopped() {
            return Err("stopped while creating the file".to_string());
        }
        for word in chunk.as_chunks_mut::<8>().0 {
            seed = xorshift(seed);
            *word = seed.to_le_bytes();
        }
        out.write_all(&chunk).map_err(|e| e.to_string())?;
        written += WRITE_CHUNK as u64;
    }
    out.sync_all().map_err(|e| e.to_string())
}

/// Uncached random block reads from `threads` threads for `seconds`; `burst` alternates on and off periods.
pub fn read_load(
    file: &Path,
    len: u64,
    seconds: u64,
    threads: usize,
    burst: Option<(Duration, Duration)>,
) -> Result<(), String> {
    let blocks = len / BLOCK as u64;
    if blocks == 0 {
        return Err(format!("{} is smaller than one block", file.display()));
    }
    println!(
        "Reading random blocks of {} from {threads} threads for {seconds} s.",
        file.display()
    );
    let start = Instant::now();
    let deadline = start + Duration::from_secs(seconds);
    let workers: Vec<_> = (0..threads)
        .map(|index| {
            let file = file.to_path_buf();
            thread::spawn(move || -> Result<u64, String> {
                let handle = OpenOptions::new()
                    .read(true)
                    .custom_flags(FILE_FLAG_NO_BUFFERING.0)
                    .open(&file)
                    .map_err(|e| e.to_string())?;
                let mut buf = Box::new(Block([0; BLOCK]));
                let mut seed = xorshift(0x2545_F491_4F6C_DD1D ^ (index as u64 + 1));
                let mut reads = 0u64;
                while Instant::now() < deadline && !stopped() {
                    if burst.is_some_and(|(on, off)| in_off_period(start.elapsed(), on, off)) {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    seed = xorshift(seed);
                    let offset = (seed % blocks) * BLOCK as u64;
                    handle
                        .seek_read(&mut buf.0, offset)
                        .map_err(|e| e.to_string())?;
                    reads += 1;
                }
                Ok(reads)
            })
        })
        .collect();
    let mut reads = 0u64;
    for worker in workers {
        reads += worker
            .join()
            .map_err(|_| "a reader thread panicked".to_string())??;
    }
    let secs = start.elapsed().as_secs_f64().max(0.001);
    println!(
        "Done: {reads} reads, {:.0} per second.",
        reads as f64 / secs
    );
    Ok(())
}

fn in_off_period(elapsed: Duration, on: Duration, off: Duration) -> bool {
    let cycle = (on + off).as_millis().max(1);
    elapsed.as_millis() % cycle >= on.as_millis()
}

fn xorshift(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}
