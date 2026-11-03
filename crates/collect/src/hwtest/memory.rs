use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vestigium_core::health::Subject;
use vestigium_core::hwtest::{
    ERRORS_KEPT, MemoryError, MemoryQuick, Outcome, PATTERNS, Pattern, QUICK_CHUNK, TestData,
    TestRun,
};
use windows::Win32::System::Memory::{
    GetProcessWorkingSetSizeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
    QUOTA_LIMITS_HARDWS_MAX_DISABLE, QUOTA_LIMITS_HARDWS_MIN_DISABLE,
    SETPROCESSWORKINGSETSIZEEX_FLAGS, SetProcessWorkingSetSizeEx, VirtualAlloc, VirtualFree,
    VirtualLock, VirtualUnlock,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
    THREAD_MODE_BACKGROUND_END, THREAD_PRIORITY_LOWEST, THREAD_PRIORITY_NORMAL,
};

use crate::{CollectError, memory_info, wall_ms};

/// Words between two checks for the stop flag and the deadline: 1 MiB.
const STEP: usize = 1 << 17;
/// Working set kept above the locked memory, for the rest of the process.
const MARGIN: usize = 64 << 20;

/// Memory a test reads and writes one 64-bit word at a time.
pub trait Words {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn put(&mut self, index: usize, value: u64);
    fn get(&self, index: usize) -> u64;
}

impl Words for [u64] {
    fn len(&self) -> usize {
        <[u64]>::len(self)
    }

    fn put(&mut self, index: usize, value: u64) {
        // SAFETY: an in-bounds, aligned word; volatile so every store reaches memory.
        unsafe { std::ptr::write_volatile(&mut self[index], value) }
    }

    fn get(&self, index: usize) -> u64 {
        // SAFETY: an in-bounds, aligned word; volatile so every load reads memory.
        unsafe { std::ptr::read_volatile(&self[index]) }
    }
}

/// Where a running test stands, after each pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryProgress {
    pub pattern: Pattern,
    pub rounds: u32,
    pub passes: u32,
    pub error_count: u64,
}

struct Halt;

struct Engine<'a, W: Words + ?Sized> {
    mem: &'a mut W,
    stop: &'a AtomicBool,
    deadline: Instant,
    pattern: Pattern,
    errors: Vec<MemoryError>,
    error_count: u64,
}

fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

impl<W: Words + ?Sized> Engine<'_, W> {
    fn halted(&self) -> bool {
        self.stop.load(Ordering::Relaxed) || Instant::now() >= self.deadline
    }

    fn found(&mut self, index: usize, expected: u64, actual: u64) {
        self.error_count += 1;
        if self.errors.len() < ERRORS_KEPT {
            self.errors.push(MemoryError {
                offset: index as u64 * 8,
                pattern: self.pattern,
                expected,
                actual,
            });
        }
    }

    fn fill(&mut self, value: impl Fn(usize) -> u64) -> Result<(), Halt> {
        let len = self.mem.len();
        for start in (0..len).step_by(STEP) {
            if self.halted() {
                return Err(Halt);
            }
            for i in start..(start + STEP).min(len) {
                self.mem.put(i, value(i));
            }
        }
        Ok(())
    }

    fn verify(&mut self, value: impl Fn(usize) -> u64) -> Result<(), Halt> {
        let len = self.mem.len();
        for start in (0..len).step_by(STEP) {
            if self.halted() {
                return Err(Halt);
            }
            for i in start..(start + STEP).min(len) {
                let (expected, actual) = (value(i), self.mem.get(i));
                if actual != expected {
                    self.found(i, expected, actual);
                }
            }
        }
        Ok(())
    }

    fn write_and_check(&mut self, value: impl Fn(usize) -> u64 + Copy) -> Result<(), Halt> {
        self.fill(value)?;
        self.verify(value)
    }

    /// Two halves filled alike, changed alike, then compared word by word.
    fn compare(&mut self, seed: u64) -> Result<(), Halt> {
        let half = self.mem.len() / 2;
        self.fill(|i| mix(seed ^ (i % half.max(1)) as u64))?;
        for (step, op) in [0u64, 1, 2].into_iter().enumerate() {
            let q = mix(seed.wrapping_add(step as u64 + 1)) | 1;
            for start in (0..2 * half).step_by(STEP) {
                if self.halted() {
                    return Err(Halt);
                }
                for i in start..(start + STEP).min(2 * half) {
                    let v = self.mem.get(i);
                    let changed = match op {
                        0 => v ^ q,
                        1 => v.wrapping_sub(q),
                        _ => v.wrapping_mul(q),
                    };
                    self.mem.put(i, changed);
                }
            }
            for start in (0..half).step_by(STEP) {
                if self.halted() {
                    return Err(Halt);
                }
                for i in start..(start + STEP).min(half) {
                    let (a, b) = (self.mem.get(i), self.mem.get(half + i));
                    if a != b {
                        self.found(half + i, a, b);
                    }
                }
            }
        }
        Ok(())
    }

    fn pattern(&mut self, pattern: Pattern, round: u32) -> Result<(), Halt> {
        self.pattern = pattern;
        let seed = mix(u64::from(round) ^ 0x5eed);
        match pattern {
            Pattern::OwnAddress => {
                self.write_and_check(|i| i as u64 * 8)?;
                self.write_and_check(|i| !(i as u64 * 8))
            }
            Pattern::Random => self.write_and_check(|i| mix(seed ^ i as u64)),
            Pattern::Compare => self.compare(seed),
            Pattern::SolidBits | Pattern::Checkerboard => {
                let base = if pattern == Pattern::SolidBits {
                    0
                } else {
                    0x5555_5555_5555_5555
                };
                for flip in [!round.is_multiple_of(2), round.is_multiple_of(2)] {
                    let v = if flip { !base } else { base };
                    self.write_and_check(|i| if i % 2 == 0 { v } else { !v })?;
                }
                Ok(())
            }
            Pattern::WalkingOnes | Pattern::WalkingZeros => {
                for b in 0..8 {
                    let one = 1u64 << ((round * 8 + b) % 64);
                    let v = if pattern == Pattern::WalkingOnes {
                        one
                    } else {
                        !one
                    };
                    self.write_and_check(|i| if i % 2 == 0 { v } else { !v })?;
                }
                Ok(())
            }
        }
    }
}

/// What the patterns covered on `mem` until `deadline` or `stop`.
pub struct PatternRun {
    pub rounds: u32,
    pub passes: u32,
    pub error_count: u64,
    pub errors: Vec<MemoryError>,
    pub stopped: bool,
}

/// Runs every pattern on `mem`, round after round, until `deadline` passes or `stop` is set.
pub fn run_patterns<W: Words + ?Sized>(
    mem: &mut W,
    deadline: Instant,
    stop: &AtomicBool,
    progress: &mut dyn FnMut(MemoryProgress),
) -> PatternRun {
    let mut engine = Engine {
        mem,
        stop,
        deadline,
        pattern: PATTERNS[0],
        errors: Vec::new(),
        error_count: 0,
    };
    let (mut rounds, mut passes) = (0u32, 0u32);
    'rounds: loop {
        for pattern in PATTERNS {
            if engine.pattern(pattern, rounds).is_err() {
                break 'rounds;
            }
            passes += 1;
            progress(MemoryProgress {
                pattern,
                rounds,
                passes,
                error_count: engine.error_count,
            });
        }
        rounds += 1;
    }
    PatternRun {
        rounds,
        passes,
        error_count: engine.error_count,
        errors: engine.errors,
        stopped: stop.load(Ordering::Relaxed),
    }
}

/// Memory allocated and locked into RAM in pieces, so the test reaches the chips and not the page
/// file; the working set bounds it raised are put back when dropped.
pub struct LockedRegion {
    base: *mut u64,
    locked: usize,
    previous: (usize, usize),
}

// SAFETY: the region is owned memory; nothing else holds its address.
unsafe impl Send for LockedRegion {}

impl LockedRegion {
    /// Locks up to `bytes`; stops at the first piece Windows refuses and keeps what it locked.
    pub fn lock(bytes: u64, stop: &AtomicBool) -> Result<LockedRegion, CollectError> {
        let reserved = usize::try_from(bytes).unwrap_or(usize::MAX) & !(QUICK_CHUNK as usize - 1);
        let (mut min, mut max, mut flags) = (0usize, 0usize, 0u32);
        // SAFETY: the pseudo-handle of this process and three out-parameters.
        unsafe { GetProcessWorkingSetSizeEx(GetCurrentProcess(), &mut min, &mut max, &mut flags) }
            .ok()?;
        // SAFETY: a fresh read-write region, released in `Drop`.
        let base =
            unsafe { VirtualAlloc(None, reserved, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };
        if base.is_null() {
            return Err(windows::core::Error::from_thread().into());
        }
        let mut region = LockedRegion {
            base: base.cast(),
            locked: 0,
            previous: (min, max),
        };
        let soft = SETPROCESSWORKINGSETSIZEEX_FLAGS(
            QUOTA_LIMITS_HARDWS_MIN_DISABLE.0 | QUOTA_LIMITS_HARDWS_MAX_DISABLE.0,
        );
        while region.locked < reserved && !stop.load(Ordering::Relaxed) {
            let next = region.locked + QUICK_CHUNK as usize;
            let raised = (min.max(MARGIN) + next, max.max(MARGIN) + next + MARGIN);
            // SAFETY: raises this process's own working set bounds; both stay soft.
            let grew = unsafe {
                SetProcessWorkingSetSizeEx(GetCurrentProcess(), raised.0, raised.1, soft)
            };
            if grew.is_err() {
                break;
            }
            // SAFETY: the piece lies inside the region allocated above.
            let piece = unsafe { region.base.cast::<u8>().add(region.locked) };
            // SAFETY: locks one whole piece of that region.
            if unsafe { VirtualLock(piece.cast(), QUICK_CHUNK as usize) }.is_err() {
                break;
            }
            region.locked = next;
        }
        if region.locked == 0 {
            return Err(windows::core::Error::from_thread().into());
        }
        Ok(region)
    }

    pub fn bytes(&self) -> u64 {
        self.locked as u64
    }

    pub fn words(&mut self) -> &mut [u64] {
        // SAFETY: the locked prefix of a region this value owns, borrowed mutably once.
        unsafe { std::slice::from_raw_parts_mut(self.base, self.locked / 8) }
    }
}

impl Drop for LockedRegion {
    fn drop(&mut self) {
        // SAFETY: unlocks and releases the region allocated in `lock`, once, and puts the
        // process's working set bounds back as they were.
        unsafe {
            if self.locked > 0 {
                let _ = VirtualUnlock(self.base.cast(), self.locked);
            }
            let _ = VirtualFree(self.base.cast(), 0, MEM_RELEASE);
            let _ = SetProcessWorkingSetSizeEx(
                GetCurrentProcess(),
                self.previous.0,
                self.previous.1,
                SETPROCESSWORKINGSETSIZEEX_FLAGS(
                    QUOTA_LIMITS_HARDWS_MIN_DISABLE.0 | QUOTA_LIMITS_HARDWS_MAX_DISABLE.0,
                ),
            );
        }
    }
}

/// Puts the calling thread in background mode at the lowest priority until dropped.
struct Background;

impl Background {
    fn begin() -> Background {
        // SAFETY: the pseudo-handle of the calling thread.
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_LOWEST);
        }
        Background
    }
}

impl Drop for Background {
    fn drop(&mut self) {
        // SAFETY: the pseudo-handle of the thread that began background mode.
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_NORMAL);
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_END);
        }
    }
}

/// A quick test of `bytes` of free memory for `length`, on the calling thread in background mode.
pub fn memory_test(
    bytes: u64,
    length: Duration,
    stop: &AtomicBool,
    progress: &mut dyn FnMut(MemoryProgress),
) -> TestRun {
    let _background = Background::begin();
    let started_ms = wall_ms();
    let total_bytes = memory_info().map_or(0, |m| m.total_bytes);
    let mut quick = MemoryQuick {
        total_bytes,
        tested_bytes: 0,
        rounds: 0,
        passes: 0,
        error_count: 0,
        errors: Vec::new(),
        failure: None,
    };
    let outcome = match LockedRegion::lock(bytes, stop) {
        Err(err) => {
            quick.failure = Some(err.to_string());
            Outcome::Failed
        }
        Ok(mut region) => {
            quick.tested_bytes = region.bytes();
            let run = run_patterns(region.words(), Instant::now() + length, stop, progress);
            quick.rounds = run.rounds;
            quick.passes = run.passes;
            quick.error_count = run.error_count;
            quick.errors = run.errors;
            if run.error_count > 0 {
                Outcome::Problems
            } else if run.stopped {
                Outcome::Interrupted
            } else {
                Outcome::Clean
            }
        }
    };
    TestRun {
        subject: Subject::Pc,
        started_ms,
        ended_ms: wall_ms(),
        outcome,
        data: TestData::MemoryQuick(quick),
    }
}
