//! A mapped ring of compact records written once a second and flushed every N seconds, to measure
//! the disk writes it costs: `blackbox_cost --path <file> --flush-secs <s> --minutes <m>`.

use std::ffi::c_void;
use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::{ProcessSnapshotter, wall_ms};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, OPEN_ALWAYS,
};
use windows::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_WRITE, FlushViewOfFile, MapViewOfFile, PAGE_READWRITE,
    UnmapViewOfFile,
};
use windows::core::HSTRING;

const SLOTS: usize = 120;
/// A compact second: time, the reading, the open episodes and a share of the top processes.
const RECORD: usize = 64;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = arg("--path").ok_or("--path is required")?;
    let flush_secs: u64 = arg("--flush-secs")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let minutes: u64 = arg("--minutes").and_then(|s| s.parse().ok()).unwrap_or(10);
    let size = (SLOTS * RECORD).next_multiple_of(4096);
    println!("ring {SLOTS} x {RECORD} B ({size} B), flush every {flush_secs} s, {minutes} min");

    // SAFETY: opens or creates the ring file; the handle is closed below.
    let file = unsafe {
        CreateFileW(
            &HSTRING::from(path.as_str()),
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_READ,
            None,
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )?
    };
    // SAFETY: maps `size` bytes of the file just opened; both handles are closed below.
    let mapping = unsafe { CreateFileMappingW(file, None, PAGE_READWRITE, 0, size as u32, None)? };
    // SAFETY: a writable view of the whole mapping, unmapped below.
    let view = unsafe { MapViewOfFile(mapping, FILE_MAP_WRITE, 0, 0, size) };
    if view.Value.is_null() {
        return Err("MapViewOfFile failed".into());
    }
    let base = view.Value.cast::<u8>();

    let mut snapshotter = ProcessSnapshotter::with_capacity(1 << 20);
    let own = |s: &mut ProcessSnapshotter| {
        s.snapshot()
            .ok()
            .and_then(|all| all.into_iter().find(|p| p.pid == std::process::id()))
            .map(|p| p.write_bytes)
            .unwrap_or(0)
    };
    let written_before = own(&mut snapshotter);
    let started = Instant::now();
    let mut flushes = 0u32;
    let mut flush_ms = 0f64;
    for tick in 0..minutes * 60 {
        let slot = (tick as usize) % SLOTS;
        let mut record = [0u8; RECORD];
        record[..8].copy_from_slice(&wall_ms().to_le_bytes());
        record[8..16].copy_from_slice(&tick.to_le_bytes());
        // SAFETY: the slot lies inside the mapped view, which stays mapped until the end.
        unsafe { std::ptr::copy_nonoverlapping(record.as_ptr(), base.add(slot * RECORD), RECORD) };
        if (tick + 1) % flush_secs == 0 {
            let at = Instant::now();
            // SAFETY: flushes the dirty pages of the live view.
            unsafe { FlushViewOfFile(view.Value as *const c_void, 0)? };
            flush_ms += at.elapsed().as_secs_f64() * 1000.0;
            flushes += 1;
        }
        let next = Duration::from_secs(tick + 1);
        thread::sleep(next.saturating_sub(started.elapsed()));
    }
    let written = own(&mut snapshotter).saturating_sub(written_before);
    println!(
        "{flushes} flushes, {:.3} ms each on average; process write counter +{written} bytes",
        flush_ms / f64::from(flushes.max(1))
    );
    // SAFETY: unmaps the view and closes both handles opened above, once each.
    unsafe {
        UnmapViewOfFile(view)?;
        CloseHandle(mapping)?;
        CloseHandle(file)?;
    }
    Ok(())
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}
