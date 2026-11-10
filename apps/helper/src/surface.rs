use std::ffi::c_void;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use vestigium_collect::{disk_media, wall_ms};
use vestigium_core::Media;
use vestigium_core::health::Subject;
use vestigium_core::hwtest::{
    Outcome, SLOW_BLOCK_MS, SURFACE_BLOCK, SurfaceAggregator, SurfaceProgress, SurfaceScan,
    TEST_RUN_VERSION, TestData, TestRun,
};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_BEGIN, FILE_FLAG_NO_BUFFERING, FILE_IO_PRIORITY_HINT_INFO, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FileIoPriorityHintInfo, IoPriorityHintVeryLow, OPEN_EXISTING, ReadFile,
    SetFileInformationByHandle, SetFilePointerEx,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    GET_LENGTH_INFORMATION, IOCTL_DISK_GET_LENGTH_INFO, IOCTL_STORAGE_QUERY_PROPERTY,
    PropertyStandardQuery, STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
    StorageAccessAlignmentProperty,
};
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc, VirtualFree,
};
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
};
use windows::core::HSTRING;

use crate::{job, output};

/// The only access the scan asks for: it never writes to the disk it reads.
pub const DISK_ACCESS: u32 = GENERIC_READ.0;
/// How often the progress file is rewritten.
const PROGRESS_EVERY_MS: f64 = 5_000.0;
/// Sectors of one block narrowed down before the rest of the block counts as unreadable too.
const SECTORS_NARROWED: usize = 64;
/// Win32 errors a disk that was removed or stopped answering returns.
const DEVICE_GONE: [u32; 7] = [6, 21, 55, 433, 1006, 1112, 1167];
/// Blocks in a row that fail whole before the disk is asked whether it still answers.
const FAILING_STREAK: usize = 3;
/// Why a scan ended when the disk went away.
const GONE_REASON: &str = "the disk stopped answering";

/// A disk read in place, block by block.
pub trait Disk {
    fn bytes(&self) -> u64;
    fn sector_bytes(&self) -> u32;
    /// Reads `len` bytes at `offset`, both multiples of the sector size; `Err` carries the Win32 error.
    fn read(&mut self, offset: u64, len: u32) -> Result<(), u32>;
    /// Whether the disk still reports its length; read-only, and false once it is gone.
    fn answers(&mut self) -> bool;
}

/// A physical drive opened for reading only, with I/O at the lowest priority.
pub struct PhysicalDisk {
    handle: HANDLE,
    bytes: u64,
    sector: u32,
    buffer: *mut u8,
}

impl PhysicalDisk {
    pub fn open(number: u32) -> Result<PhysicalDisk, String> {
        let path = HSTRING::from(format!(r"\\.\PhysicalDrive{number}"));
        // SAFETY: opens the device for reading only; `Drop` closes the handle.
        let handle = unsafe {
            CreateFileW(
                &path,
                DISK_ACCESS,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_NO_BUFFERING,
                None,
            )
        }
        .map_err(|e| e.to_string())?;
        // SAFETY: a fresh read-write buffer of one block, released in `Drop`.
        let buffer = unsafe {
            VirtualAlloc(
                None,
                SURFACE_BLOCK as usize,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        let mut disk = PhysicalDisk {
            handle,
            bytes: 0,
            sector: 512,
            buffer: buffer.cast(),
        };
        if buffer.is_null() {
            return Err("no memory for the read buffer".into());
        }
        let mut length = GET_LENGTH_INFORMATION::default();
        disk.ask(
            IOCTL_DISK_GET_LENGTH_INFO,
            &[],
            (&raw mut length).cast(),
            size_of_val(&length),
        )
        .map_err(|e| e.to_string())?;
        disk.bytes = length.Length as u64;
        let query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageAccessAlignmentProperty,
            QueryType: PropertyStandardQuery,
            ..Default::default()
        };
        let mut alignment = STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR::default();
        // SAFETY: the query is a live struct whose bytes are passed whole.
        let input = unsafe {
            std::slice::from_raw_parts((&raw const query).cast::<u8>(), size_of_val(&query))
        };
        if disk
            .ask(
                IOCTL_STORAGE_QUERY_PROPERTY,
                input,
                (&raw mut alignment).cast(),
                size_of_val(&alignment),
            )
            .is_ok()
            && alignment.BytesPerLogicalSector >= 512
        {
            disk.sector = alignment.BytesPerLogicalSector;
        }
        let hint = FILE_IO_PRIORITY_HINT_INFO {
            PriorityHint: IoPriorityHintVeryLow,
        };
        // SAFETY: a live hint struct of the size passed, on the handle this value owns.
        let _ = unsafe {
            SetFileInformationByHandle(
                disk.handle,
                FileIoPriorityHintInfo,
                (&raw const hint).cast::<c_void>(),
                size_of_val(&hint) as u32,
            )
        };
        Ok(disk)
    }

    fn ask(
        &self,
        code: u32,
        input: &[u8],
        out: *mut c_void,
        out_len: usize,
    ) -> windows::core::Result<()> {
        let mut returned = 0u32;
        // SAFETY: the input slice and the output buffer of `out_len` bytes are live for the call.
        unsafe {
            DeviceIoControl(
                self.handle,
                code,
                (!input.is_empty()).then_some(input.as_ptr().cast::<c_void>()),
                input.len() as u32,
                Some(out),
                out_len as u32,
                Some(&mut returned),
                None,
            )
        }
    }
}

impl Disk for PhysicalDisk {
    fn bytes(&self) -> u64 {
        self.bytes
    }

    fn sector_bytes(&self) -> u32 {
        self.sector
    }

    fn read(&mut self, offset: u64, len: u32) -> Result<(), u32> {
        let code = |e: windows::core::Error| (e.code().0 as u32) & 0xFFFF;
        // SAFETY: moves the file pointer of the handle this value owns.
        unsafe { SetFilePointerEx(self.handle, offset as i64, None, FILE_BEGIN) }.map_err(code)?;
        // SAFETY: the page-aligned buffer holds one block, and `len` is at most one block.
        let buf =
            unsafe { std::slice::from_raw_parts_mut(self.buffer, len.min(SURFACE_BLOCK) as usize) };
        let mut read = 0u32;
        // SAFETY: a synchronous read into the buffer above.
        unsafe { ReadFile(self.handle, Some(buf), Some(&mut read), None) }.map_err(code)?;
        if read == len { Ok(()) } else { Err(0) }
    }

    fn answers(&mut self) -> bool {
        let mut length = GET_LENGTH_INFORMATION::default();
        self.ask(
            IOCTL_DISK_GET_LENGTH_INFO,
            &[],
            (&raw mut length).cast(),
            size_of_val(&length),
        )
        .is_ok()
    }
}

impl Drop for PhysicalDisk {
    fn drop(&mut self) {
        // SAFETY: releases the buffer and closes the handle `open` made, once each.
        unsafe {
            if !self.buffer.is_null() {
                let _ = VirtualFree(self.buffer.cast(), 0, MEM_RELEASE);
            }
            let _ = CloseHandle(self.handle);
        }
    }
}

/// Reads `disk` from `from_bytes` to its end, or until `stop` says so; a slow block is read again
/// and counts by its faster time, and a block that fails is narrowed down to its sectors.
pub fn scan(
    disk: &mut impl Disk,
    number: u32,
    media: Media,
    from_bytes: u64,
    stop: &mut dyn FnMut() -> bool,
    progress: &mut dyn FnMut(SurfaceProgress),
) -> (SurfaceScan, bool) {
    let (size, sector) = (disk.bytes(), disk.sector_bytes().max(1));
    let block = u64::from(SURFACE_BLOCK);
    let mut offset = (from_bytes.min(size) / block) * block;
    let mut scan = SurfaceAggregator::new(number, media, size, sector, offset);
    let clock = Instant::now();
    let ms = |since: Instant| since.elapsed().as_secs_f64() * 1000.0;
    let mut last_progress = 0.0;
    let mut stopped = false;
    let mut gone = false;
    // Blocks that failed whole and are not recorded until the disk is known to still answer.
    let mut streak: Vec<(u64, Vec<u64>)> = Vec::new();
    while offset < size {
        if stop() {
            stopped = true;
            break;
        }
        let len = (size - offset).min(block) as u32 / sector * sector;
        if len == 0 {
            break;
        }
        let started = Instant::now();
        let first = disk.read(offset, len);
        let mut took = ms(started) as f32;
        if first.is_ok() && took >= SLOW_BLOCK_MS {
            let again = Instant::now();
            if disk.read(offset, len).is_ok() {
                took = took.min(ms(again) as f32);
            }
        }
        if first.is_ok() {
            flush(&mut scan, &mut streak);
            scan.block(offset, took);
        } else {
            let mut bad = Vec::new();
            let mut vanished = first.is_err_and(|code| DEVICE_GONE.contains(&code));
            for at in (offset..offset + u64::from(len)).step_by(sector as usize) {
                if vanished {
                    break;
                }
                if let Err(code) = disk.read(at, sector) {
                    vanished = DEVICE_GONE.contains(&code);
                    bad.push(at);
                    if bad.len() >= SECTORS_NARROWED {
                        break;
                    }
                }
            }
            if vanished {
                gone = true;
                break;
            }
            if bad.is_empty() {
                flush(&mut scan, &mut streak);
                scan.block(offset, took);
            } else if bad.len() >= SECTORS_NARROWED {
                streak.push((offset, bad));
                if streak.len() >= FAILING_STREAK {
                    if !disk.answers() {
                        gone = true;
                        break;
                    }
                    flush(&mut scan, &mut streak);
                }
            } else {
                flush(&mut scan, &mut streak);
                scan.unreadable(offset, &bad);
            }
        }
        offset += u64::from(len);
        if ms(clock) - last_progress >= PROGRESS_EVERY_MS {
            last_progress = ms(clock);
            progress(scan.progress(wall_ms(), offset));
        }
    }
    if gone {
        // A later scan resumes at the first block that was not recorded.
        let resume = streak.first().map_or(offset, |(at, _)| *at);
        return (scan.finish(resume, Some(GONE_REASON.into())), stopped);
    }
    flush(&mut scan, &mut streak);
    (scan.finish(offset, None), stopped)
}

/// Records the blocks held back as unreadable once the disk is known to answer.
fn flush(scan: &mut SurfaceAggregator, streak: &mut Vec<(u64, Vec<u64>)>) {
    for (at, bad) in streak.drain(..) {
        scan.unreadable(at, &bad);
    }
}

/// The run a scan makes, with its outcome.
pub fn as_run(scan: SurfaceScan, stopped: bool, started_ms: i64) -> TestRun {
    let outcome = if scan.failure.is_some() {
        Outcome::Failed
    } else if scan.unreadable_blocks > 0 {
        Outcome::Problems
    } else if stopped || scan.to_bytes < scan.disk_bytes {
        Outcome::Interrupted
    } else {
        Outcome::Clean
    };
    TestRun {
        subject: Subject::Disk(scan.disk.to_string()),
        started_ms,
        ended_ms: wall_ms(),
        outcome,
        data: TestData::DiskSurface(scan),
    }
}

/// A run that could not read the disk at all.
pub fn failed_run(number: u32, media: Media, reason: String, started_ms: i64) -> TestRun {
    let scan = SurfaceAggregator::new(number, media, 0, 512, 0).finish(0, Some(reason));
    as_run(scan, false, started_ms)
}

/// Writes a run as the agent reads it: the version byte, then the run.
pub(crate) fn write_run(file: &mut File, run: &TestRun) -> bool {
    let mut bytes = vec![TEST_RUN_VERSION];
    match postcard::to_allocvec(run) {
        Ok(body) => bytes.extend(body),
        Err(_) => return false,
    }
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .is_ok()
}

fn rewrite(file: &mut File, progress: &SurfaceProgress) {
    let mut bytes = vec![TEST_RUN_VERSION];
    if let Ok(body) = postcard::to_allocvec(progress) {
        bytes.extend(body);
        let _ = file
            .seek(SeekFrom::Start(0))
            .and_then(|_| file.write_all(&bytes))
            .and_then(|()| file.set_len(bytes.len() as u64));
    }
}

/// Scans physical drive `number` from `from_mib` into a new `.surface` file, rewriting a
/// `.progress` file beside it and stopping when a `.stop` file appears there.
pub fn run(number: u32, from_mib: u64, out: &Path) -> ExitCode {
    let Ok(mut file) = output::create(out, "tests", "surface") else {
        return ExitCode::from(job::REFUSED);
    };
    let Ok(mut progress_file) =
        output::create(&out.with_extension("progress"), "tests", "progress")
    else {
        return ExitCode::from(job::REFUSED);
    };
    let stop_path = out.with_extension("stop");
    let started_ms = wall_ms();
    let media = disk_media(number);
    // SAFETY: the pseudo-handle of this thread; background mode lowers its I/O priority too.
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
    }
    let run = match PhysicalDisk::open(number) {
        Err(reason) => failed_run(number, media, reason, started_ms),
        Ok(mut disk) => {
            let (scan, stopped) = scan(
                &mut disk,
                number,
                media,
                from_mib.saturating_mul(1 << 20),
                &mut || stop_path.exists(),
                &mut |p| rewrite(&mut progress_file, &p),
            );
            as_run(scan, stopped, started_ms)
        }
    };
    let _ = output::delete_through(&progress_file);
    drop(progress_file);
    if write_run(&mut file, &run) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(job::WRITE_FAILED)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use vestigium_core::hwtest::combined_surface;
    use windows::Win32::Foundation::GENERIC_WRITE;

    use super::*;

    const MIB: u64 = 1 << 20;

    /// A disk in memory: one sector that never reads, and blocks that read slowly the first time.
    struct FakeDisk {
        bytes: u64,
        bad_sector: Option<u64>,
        slow_once: Vec<u64>,
        reads: u32,
        /// Every read from this offset on fails with this Win32 error.
        fails_from: Option<(u64, u32)>,
        /// What the length query says once reads fail.
        answers: bool,
    }

    impl FakeDisk {
        fn new(mib: u64) -> FakeDisk {
            FakeDisk {
                bytes: mib * MIB,
                bad_sector: None,
                slow_once: Vec::new(),
                reads: 0,
                fails_from: None,
                answers: true,
            }
        }
    }

    impl Disk for FakeDisk {
        fn bytes(&self) -> u64 {
            self.bytes
        }

        fn sector_bytes(&self) -> u32 {
            512
        }

        fn read(&mut self, offset: u64, len: u32) -> Result<(), u32> {
            self.reads += 1;
            if let Some(at) = self.slow_once.iter().position(|&o| o == offset) {
                self.slow_once.remove(at);
                std::thread::sleep(Duration::from_millis(600));
            }
            if let Some((from, code)) = self.fails_from
                && offset + u64::from(len) > from
            {
                return Err(code);
            }
            match self.bad_sector {
                Some(bad) if (offset..offset + u64::from(len)).contains(&bad) => Err(23),
                _ => Ok(()),
            }
        }

        fn answers(&mut self) -> bool {
            self.answers
        }
    }

    fn scan_all(disk: &mut FakeDisk, from: u64) -> (SurfaceScan, bool) {
        scan(disk, 1, Media::Hdd, from, &mut || false, &mut |_| {})
    }

    #[test]
    fn never_opens_the_disk_for_writing() {
        assert_eq!(DISK_ACCESS, GENERIC_READ.0);
        assert_eq!(DISK_ACCESS & GENERIC_WRITE.0, 0);
    }

    #[test]
    fn reads_the_whole_disk_block_by_block() {
        let mut disk = FakeDisk::new(64);
        let (scan, stopped) = scan_all(&mut disk, 0);
        assert!(!stopped && scan.whole_disk());
        assert_eq!(scan.bands.iter().sum::<u64>(), 64);
        assert_eq!(as_run(scan, false, 0).outcome, Outcome::Clean);
    }

    #[test]
    fn stop_file_ends_with_a_partial_map() {
        let mut disk = FakeDisk::new(64);
        let mut asked = 0;
        let (scan, stopped) = scan(
            &mut disk,
            1,
            Media::Hdd,
            0,
            &mut || {
                asked += 1;
                asked > 10
            },
            &mut |_| {},
        );
        assert!(stopped);
        assert_eq!(scan.to_bytes, 10 * MIB);
        assert_eq!(as_run(scan, true, 0).outcome, Outcome::Interrupted);
    }

    #[test]
    fn resumes_from_the_stop_point() {
        let mut disk = FakeDisk::new(64);
        let (first, _) = scan(&mut disk, 1, Media::Hdd, 0, &mut || false, &mut |_| {});
        let first = SurfaceScan {
            to_bytes: 32 * MIB,
            ..first
        };
        let (second, stopped) = scan_all(&mut disk, 32 * MIB + 5);
        assert!(!stopped);
        assert_eq!(second.from_bytes, 32 * MIB);
        assert_eq!(second.bands.iter().sum::<u64>(), 32);
        assert!(combined_surface(&[first, second]).unwrap().whole_disk());
    }

    #[test]
    fn a_read_error_is_narrowed_to_sectors() {
        let mut disk = FakeDisk::new(8);
        let bad = 3 * MIB + 7 * 512;
        disk.bad_sector = Some(bad);
        let (scan, _) = scan_all(&mut disk, 0);
        assert_eq!(scan.unreadable_blocks, 1);
        assert_eq!(scan.unreadable, vec![bad]);
        assert_eq!(scan.bands.iter().sum::<u64>(), 7);
        assert_eq!(as_run(scan, false, 0).outcome, Outcome::Problems);
    }

    #[test]
    fn a_disk_that_vanishes_ends_the_scan_as_failed() {
        let mut disk = FakeDisk::new(8);
        disk.fails_from = Some((3 * MIB, 1167));
        let (scan, _) = scan_all(&mut disk, 0);
        assert_eq!(scan.unreadable_blocks, 0);
        assert_eq!(
            scan.to_bytes,
            3 * MIB,
            "a later scan resumes at the failed block"
        );
        assert!(scan.failure.is_some());
        assert_eq!(as_run(scan, false, 0).outcome, Outcome::Failed);
    }

    #[test]
    fn a_silent_disk_ends_the_scan_as_failed_without_unreadable_blocks() {
        let mut disk = FakeDisk::new(12);
        disk.fails_from = Some((2 * MIB, 23));
        disk.answers = false;
        let (scan, _) = scan_all(&mut disk, 0);
        assert_eq!(scan.unreadable_blocks, 0);
        assert_eq!(scan.to_bytes, 2 * MIB);
        assert_eq!(as_run(scan, false, 0).outcome, Outcome::Failed);
    }

    #[test]
    fn a_damaged_area_of_a_disk_that_answers_is_still_problems() {
        let mut disk = FakeDisk::new(12);
        disk.fails_from = Some((2 * MIB, 23));
        let (scan, _) = scan_all(&mut disk, 0);
        assert_eq!(scan.unreadable_blocks, 10);
        assert!(scan.failure.is_none());
        assert_eq!(as_run(scan, false, 0).outcome, Outcome::Problems);
    }

    #[test]
    fn a_plain_bad_sector_is_still_problems() {
        let mut disk = FakeDisk::new(8);
        disk.bad_sector = Some(2 * MIB + 512);
        disk.answers = false;
        let (scan, _) = scan_all(&mut disk, 0);
        assert!(scan.failure.is_none());
        assert_eq!(as_run(scan, false, 0).outcome, Outcome::Problems);
    }

    #[test]
    fn a_slow_block_counts_by_its_faster_read() {
        let mut disk = FakeDisk::new(4);
        disk.slow_once = vec![2 * MIB];
        let (scan, _) = scan_all(&mut disk, 0);
        assert_eq!(scan.slow_blocks(), 0);
        assert_eq!(disk.reads, 5, "the slow block is read twice");
    }

    #[test]
    fn a_missing_disk_is_a_failed_run() {
        let run = failed_run(9, Media::Unknown, "not found".into(), 0);
        assert_eq!(run.outcome, Outcome::Failed);
        assert_eq!(run.subject, Subject::Disk("9".into()));
    }

    #[test]
    fn output_outside_the_tests_folder_is_refused() {
        for bad in [
            r"C:\Windows\Temp\1.surface",
            r"C:\Users\someone\AppData\Local\Vestigium\data\scans\1.surface",
            r"C:\Users\someone\AppData\Local\Vestigium\data\tests\..\1.surface",
            r"C:\Users\someone\AppData\Local\Vestigium\data\tests\1.capture",
        ] {
            assert_eq!(
                run(0, 0, Path::new(bad)),
                ExitCode::from(job::REFUSED),
                "{bad}"
            );
        }
    }
}
