//! Measures, without elevation, what the hardware tests of the Tests screen cost the rest of the PC:
//! `hwtest_probe [memory] [bench] [memdiag]`, all three when none is named.

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::{HighResTimer, memory_info, monotonic_ms};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE,
};
use windows::Win32::Storage::FileSystem::{
    CREATE_NEW, CreateFileW, FILE_FLAG_DELETE_ON_CLOSE, FILE_FLAG_NO_BUFFERING,
    FILE_FLAG_OVERLAPPED, FILE_FLAG_WRITE_THROUGH, FILE_SHARE_MODE, GetDriveTypeW,
    GetLogicalDrives, ReadFile, WriteFile,
};
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtRender, EvtRenderEventXml,
};
use windows::Win32::System::IO::{CreateIoCompletionPort, GetQueuedCompletionStatus, OVERLAPPED};
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, QUOTA_LIMITS_HARDWS_MAX_DISABLE,
    QUOTA_LIMITS_HARDWS_MIN_DISABLE, SETPROCESSWORKINGSETSIZEEX_FLAGS, SetProcessWorkingSetSizeEx,
    VirtualAlloc, VirtualFree, VirtualLock, VirtualUnlock,
};
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW,
    PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
    THREAD_PRIORITY_LOWEST,
};
use windows::core::{HSTRING, PCWSTR};

const GIB: usize = 1 << 30;
const MIB: usize = 1 << 20;
const DRIVE_FIXED: u32 = 3;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let all = args.is_empty();
    if all || args.iter().any(|a| a == "memdiag") {
        memdiag();
    }
    if all || args.iter().any(|a| a == "memory") {
        memory();
    }
    if all || args.iter().any(|a| a == "bench") {
        bench();
    }
}

/// Lag of a normal-priority 16 ms timer while something else runs: (ticks, p99 ms, max ms).
struct LagProbe {
    stop: Arc<AtomicBool>,
    handle: thread::JoinHandle<Vec<f64>>,
}

impl LagProbe {
    fn start() -> LagProbe {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            let timer = HighResTimer::new().expect("timer");
            let mut lags = Vec::new();
            while !flag.load(Ordering::Relaxed) {
                let start = monotonic_ms();
                timer.wait(Duration::from_millis(16)).expect("wait");
                lags.push(monotonic_ms() - start - 16.0);
            }
            lags
        });
        LagProbe { stop, handle }
    }

    fn finish(self) -> String {
        self.stop.store(true, Ordering::Relaxed);
        let mut lags = self.handle.join().expect("probe");
        lags.sort_by(f64::total_cmp);
        let at = |q: f64| lags[((lags.len() - 1) as f64 * q) as usize];
        format!(
            "probe {} ticks, p50 {:.2} ms, p99 {:.2} ms, max {:.2} ms",
            lags.len(),
            at(0.5),
            at(0.99),
            lags.last().copied().unwrap_or_default()
        )
    }
}

/// One English PDH counter read twice, as a rate over the interval.
struct Rate {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

impl Rate {
    fn open(path: &str) -> Rate {
        let mut query = PDH_HQUERY::default();
        let mut counter = PDH_HCOUNTER::default();
        // SAFETY: the query and counter handles are out-parameters owned here.
        unsafe {
            assert_eq!(PdhOpenQueryW(PCWSTR::null(), 0, &mut query), 0);
            assert_eq!(
                PdhAddEnglishCounterW(query, &HSTRING::from(path), 0, &mut counter),
                0
            );
            PdhCollectQueryData(query);
        }
        Rate { query, counter }
    }

    fn read(&self) -> f64 {
        let mut value = PDH_FMT_COUNTERVALUE::default();
        // SAFETY: both handles come from `open`; `value` is a valid out-parameter.
        unsafe {
            PdhCollectQueryData(self.query);
            PdhGetFormattedCounterValue(self.counter, PDH_FMT_DOUBLE, None, &mut value);
            value.Anonymous.doubleValue
        }
    }
}

impl Drop for Rate {
    fn drop(&mut self) {
        // SAFETY: the query was opened in `open` and is closed once.
        unsafe { PdhCloseQuery(self.query) };
    }
}

fn memdiag() {
    println!("== memdiag");
    let started = Instant::now();
    let query =
        HSTRING::from("*[System[Provider[@Name='Microsoft-Windows-MemoryDiagnostics-Results']]]");
    // SAFETY: a forward query on the System channel; every handle is closed below.
    let Ok(results) = (unsafe {
        EvtQuery(
            None,
            &HSTRING::from("System"),
            &query,
            EvtQueryChannelPath.0,
        )
    }) else {
        println!("query failed");
        return;
    };
    let mut count = 0;
    loop {
        let mut events = [0isize; 16];
        let mut returned = 0u32;
        // SAFETY: `events` holds 16 handles and `returned` says how many were filled.
        if unsafe { EvtNext(results, &mut events, 0, 0, &mut returned) }.is_err() {
            break;
        }
        for &raw in &events[..returned as usize] {
            let event = EVT_HANDLE(raw);
            let xml = render(event);
            count += 1;
            println!("{}", fields(&xml));
            // SAFETY: each event handle from EvtNext is closed once.
            unsafe {
                let _ = EvtClose(event);
            }
        }
    }
    // SAFETY: the result set handle is closed once.
    unsafe {
        let _ = EvtClose(results);
    }
    println!(
        "{count} events in {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

fn render(event: EVT_HANDLE) -> String {
    let mut used = 0u32;
    let mut props = 0u32;
    // SAFETY: a size query with no buffer; it fails with the size needed.
    let _ = unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0,
            0,
            None,
            &mut used,
            &mut props,
        )
    };
    let mut buf = vec![0u16; used as usize / 2 + 1];
    // SAFETY: `buf` holds `used` bytes.
    let ok = unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0,
            (buf.len() * 2) as u32,
            Some(buf.as_mut_ptr().cast::<c_void>()),
            &mut used,
            &mut props,
        )
    };
    if ok.is_err() {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..used as usize / 2])
        .trim_end_matches('\0')
        .to_string()
}

fn tag<'a>(xml: &'a str, name: &str) -> &'a str {
    let open = format!("<{name}>");
    let Some(start) = xml.find(&open).map(|i| i + open.len()) else {
        return "-";
    };
    xml[start..].split('<').next().unwrap_or("-")
}

fn fields(xml: &str) -> String {
    let time = xml
        .split("SystemTime='")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .unwrap_or("-");
    let names = [
        "LaunchType",
        "CompletionType",
        "MemorySize",
        "TestType",
        "TestDuration",
        "TestCount",
        "NumPagesTested",
        "NumPagesUnTested",
        "NumBadPages",
    ];
    let mut line = format!("{} {}", tag(xml, "EventID"), time);
    for name in names {
        if xml.contains(&format!("<{name}>")) {
            line.push_str(&format!(" {name}={}", tag(xml, name)));
        }
    }
    line
}

fn memory() {
    println!("== memory");
    let info = memory_info().expect("memory info");
    let total = info.total_bytes as usize;
    let avail = info.avail_bytes as usize;
    println!(
        "total {:.1} GiB, available {:.1} GiB, in use {:.0}%",
        total as f64 / GIB as f64,
        avail as f64 / GIB as f64,
        100.0 * (total - avail) as f64 / total as f64
    );
    let quarter = (avail / 4) & !(MIB - 1);
    for size in [GIB, 4 * GIB, quarter] {
        let in_use_after = (total - avail + size) as f64 / total as f64;
        if in_use_after > 0.80 {
            println!(
                "{} MiB skipped: use would reach {:.0}%",
                size / MIB,
                in_use_after * 100.0
            );
            continue;
        }
        lock_and_test(size);
    }
}

fn lock_and_test(size: usize) {
    println!("-- {} MiB", size / MIB);
    let faults = Rate::open(r"\Memory\Page Reads/sec");
    // SAFETY: the pseudo-handle of this process; nothing to close.
    let process = unsafe { GetCurrentProcess() };
    let flags = SETPROCESSWORKINGSETSIZEEX_FLAGS(
        QUOTA_LIMITS_HARDWS_MIN_DISABLE.0 | QUOTA_LIMITS_HARDWS_MAX_DISABLE.0,
    );
    let raised =
        // SAFETY: raises this process's own working set bounds; both are soft.
        unsafe { SetProcessWorkingSetSizeEx(process, size + 64 * MIB, size + 256 * MIB, flags) };
    if let Err(e) = raised {
        println!("working set: {e}");
        return;
    }
    let started = Instant::now();
    // SAFETY: a fresh read-write region of `size` bytes, freed below.
    let base = unsafe { VirtualAlloc(None, size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };
    if base.is_null() {
        println!("alloc failed");
        return;
    }
    // SAFETY: `base` is the region just allocated, `size` bytes long.
    let locked = unsafe { VirtualLock(base, size) };
    let lock_ms = started.elapsed().as_secs_f64() * 1000.0;
    match &locked {
        Ok(()) => println!("locked in {lock_ms:.0} ms"),
        Err(e) => println!("lock failed after {lock_ms:.0} ms: {e}"),
    }
    let words = size / 8;
    let addr = base as usize;
    let probe = LagProbe::start();
    let worker = thread::spawn(move || {
        // SAFETY: the pseudo-handle of this thread; background mode lowers I/O and memory
        // priority, and the lowest priority keeps the CPU for foreground work.
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_LOWEST);
        }
        // SAFETY: the region stays allocated until this thread is joined.
        let mem = unsafe { std::slice::from_raw_parts_mut(addr as *mut u64, words) };
        patterns(mem)
    });
    let report = worker.join().expect("worker");
    println!("{}", probe.finish());
    for line in report {
        println!("{line}");
    }
    println!("page reads/s during test (system): {:.0}", faults.read());
    let started = Instant::now();
    // SAFETY: the region is unlocked and freed once, after the worker is done with it.
    unsafe {
        if locked.is_ok() {
            let _ = VirtualUnlock(base, size);
        }
        let _ = VirtualFree(base, 0, MEM_RELEASE);
        let _ = SetProcessWorkingSetSizeEx(process, usize::MAX, usize::MAX, flags);
    }
    println!(
        "released in {:.0} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

fn gbps(bytes: usize, started: Instant) -> f64 {
    bytes as f64 / GIB as f64 / started.elapsed().as_secs_f64()
}

fn put(word: &mut u64, value: u64) {
    // SAFETY: a valid, aligned reference; volatile so every store reaches memory.
    unsafe { std::ptr::write_volatile(word, value) }
}

fn get(word: &u64) -> u64 {
    // SAFETY: a valid, aligned reference; volatile so every load reads memory.
    unsafe { std::ptr::read_volatile(word) }
}

fn patterns(mem: &mut [u64]) -> Vec<String> {
    let bytes = mem.len() * 8;
    let mut out = Vec::new();
    let mut errors = 0usize;

    let started = Instant::now();
    for (i, w) in mem.iter_mut().enumerate() {
        put(w, i as u64);
    }
    for (i, w) in mem.iter().enumerate() {
        errors += usize::from(get(w) != i as u64);
    }
    out.push(format!(
        "own address: {:.2} GB/s (write+verify)",
        gbps(2 * bytes, started)
    ));

    let started = Instant::now();
    let seed = 0x9e37_79b9_7f4a_7c15u64;
    let next = |s: &mut u64| {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s
    };
    let mut s = seed;
    for w in mem.iter_mut() {
        put(w, next(&mut s));
    }
    s = seed;
    for w in mem.iter() {
        errors += usize::from(get(w) != next(&mut s));
    }
    out.push(format!(
        "random: {:.2} GB/s (write+verify)",
        gbps(2 * bytes, started)
    ));

    let started = Instant::now();
    for bit in 0..16 {
        let one = 1u64 << bit;
        for w in mem.iter_mut() {
            put(w, one);
        }
        for w in mem.iter() {
            errors += usize::from(get(w) != one);
        }
    }
    out.push(format!(
        "walking ones (16 passes): {:.2} GB/s, {:.1} s",
        gbps(32 * bytes, started),
        started.elapsed().as_secs_f64()
    ));
    out.push(format!("errors: {errors}"));
    out
}

#[derive(Clone, Copy)]
enum Mode {
    SeqWrite,
    SeqRead,
    RndRead,
    RndWrite,
}

#[repr(C)]
struct Slot {
    ov: OVERLAPPED,
    buf: *mut u8,
    issued: f64,
}

struct Run {
    bytes: u64,
    ios: u64,
    secs: f64,
    max_ms: f64,
}

fn bench() {
    println!("== bench");
    // SAFETY: reads the bitmask of logical drives.
    let mask = unsafe { GetLogicalDrives() };
    let temp = std::env::temp_dir();
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    for letter in b'A'..=b'Z' {
        if mask & (1 << (letter - b'A')) == 0 {
            continue;
        }
        let root = format!("{}:\\", letter as char);
        // SAFETY: a drive root path.
        if unsafe { GetDriveTypeW(&HSTRING::from(root.as_str())) } != DRIVE_FIXED {
            continue;
        }
        let dir = if root.starts_with(&system) {
            temp.clone()
        } else {
            root.clone().into()
        };
        bench_volume(&dir.join(format!("vestigium-bench-{}.tmp", std::process::id())));
    }
}

fn bench_volume(path: &std::path::Path) {
    println!("-- {}", path.display());
    const LEN: u64 = 256 * MIB as u64;
    // SAFETY: a new file this run owns, deleted when the handle closes.
    let opened = unsafe {
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
    let file = match opened {
        Ok(file) => file,
        Err(e) => {
            println!("cannot create: {e}");
            return;
        }
    };
    // SAFETY: binds the file to a new completion port; both handles are closed below.
    let port = unsafe { CreateIoCompletionPort(file, None, 0, 1) }.expect("port");
    let buffers = 32 * MIB;
    // SAFETY: page-aligned buffers for unbuffered I/O, freed below.
    let base =
        unsafe { VirtualAlloc(None, buffers, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) } as *mut u8;
    let mut s = 0x2545_f491_4f6c_dd1du64;
    // SAFETY: the region was just allocated with `buffers` bytes.
    let fill = unsafe { std::slice::from_raw_parts_mut(base as *mut u64, buffers / 8) };
    for w in fill.iter_mut() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        *w = s;
    }

    let probe = LagProbe::start();
    let ext = io(file, port, base, Mode::SeqWrite, LEN, MIB, 8, 30.0, LEN);
    report("extend 1M Q8", &ext);
    let runs = [
        ("seq write 1M Q8", Mode::SeqWrite, MIB, 8, LEN),
        ("seq read 1M Q8", Mode::SeqRead, MIB, 8, u64::MAX),
        ("rnd read 4K Q32", Mode::RndRead, 4096, 32, u64::MAX),
        ("rnd write 4K Q32", Mode::RndWrite, 4096, 32, LEN),
        ("rnd read 4K Q1", Mode::RndRead, 4096, 1, u64::MAX),
        ("rnd write 4K Q1", Mode::RndWrite, 4096, 1, LEN / 4),
    ];
    let mut written = ext.bytes;
    for (name, mode, block, depth, cap) in runs {
        let run = io(file, port, base, mode, LEN, block, depth, 5.0, cap);
        if matches!(mode, Mode::SeqWrite | Mode::RndWrite) {
            written += run.bytes;
        }
        report(name, &run);
    }
    println!("{}", probe.finish());
    let addr = base as usize;
    let (f, p) = (file.0 as usize, port.0 as usize);
    let background = thread::spawn(move || {
        // SAFETY: the pseudo-handle of this thread.
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
        }
        io(
            HANDLE(f as *mut c_void),
            HANDLE(p as *mut c_void),
            addr as *mut u8,
            Mode::SeqRead,
            LEN,
            MIB,
            8,
            5.0,
            u64::MAX,
        )
    })
    .join()
    .expect("background");
    report("seq read 1M Q8, background thread", &background);
    println!("written in all: {} MiB", written / MIB as u64);
    // SAFETY: closes the handles and frees the buffers once; closing deletes the file.
    unsafe {
        let _ = VirtualFree(base.cast(), 0, MEM_RELEASE);
        let _ = CloseHandle(port);
        let _ = CloseHandle(file);
    }
}

fn report(name: &str, run: &Run) {
    let mbps = run.bytes as f64 / MIB as f64 / run.secs;
    let iops = run.ios as f64 / run.secs;
    println!(
        "{name}: {mbps:.1} MB/s, {iops:.0} IOPS, max {:.1} ms, {:.1} s",
        run.max_ms, run.secs
    );
}

#[allow(clippy::too_many_arguments)]
fn io(
    file: HANDLE,
    port: HANDLE,
    buffers: *mut u8,
    mode: Mode,
    len: u64,
    block: usize,
    depth: usize,
    limit_s: f64,
    cap: u64,
) -> Run {
    let blocks = len / block as u64;
    let mut rng = 0x1234_5678_9abc_def1u64 ^ monotonic_ms().to_bits();
    let mut next_seq = 0u64;
    let mut slots: Vec<Box<Slot>> = (0..depth)
        .map(|i| {
            Box::new(Slot {
                ov: OVERLAPPED::default(),
                // SAFETY: each slot gets its own block inside the 32 MiB buffer region.
                buf: unsafe { buffers.add(i * block) },
                issued: 0.0,
            })
        })
        .collect();
    let started = monotonic_ms();
    let (mut bytes, mut ios, mut issued_bytes, mut max_ms) = (0u64, 0u64, 0u64, 0f64);
    let mut in_flight = 0usize;
    let mut issue = |slot: &mut Slot, issued_bytes: &mut u64| -> bool {
        let elapsed = (monotonic_ms() - started) / 1000.0;
        if elapsed >= limit_s || *issued_bytes >= cap {
            return false;
        }
        let offset = match mode {
            Mode::SeqWrite | Mode::SeqRead => {
                if next_seq >= blocks {
                    if matches!(mode, Mode::SeqWrite) {
                        return false;
                    }
                    next_seq = 0;
                }
                next_seq += 1;
                (next_seq - 1) * block as u64
            }
            Mode::RndRead | Mode::RndWrite => {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                (rng % blocks) * block as u64
            }
        };
        slot.ov = OVERLAPPED::default();
        slot.ov.Anonymous.Anonymous.Offset = offset as u32;
        slot.ov.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
        slot.issued = monotonic_ms();
        // SAFETY: the slot's buffer holds `block` bytes and outlives the I/O, which completes
        // on the port before the slot is reused or dropped.
        let result = unsafe {
            let buf = std::slice::from_raw_parts_mut(slot.buf, block);
            match mode {
                Mode::SeqWrite | Mode::RndWrite => {
                    WriteFile(file, Some(buf), None, Some(&mut slot.ov))
                }
                Mode::SeqRead | Mode::RndRead => {
                    ReadFile(file, Some(buf), None, Some(&mut slot.ov))
                }
            }
        };
        match result {
            Ok(()) => {}
            Err(e) if e.code() == ERROR_IO_PENDING.to_hresult() => {}
            Err(e) => panic!("I/O failed: {e}"),
        }
        *issued_bytes += block as u64;
        true
    };
    for slot in slots.iter_mut() {
        if issue(slot, &mut issued_bytes) {
            in_flight += 1;
        }
    }
    while in_flight > 0 {
        let mut done = 0u32;
        let mut key = 0usize;
        let mut ov: *mut OVERLAPPED = std::ptr::null_mut();
        // SAFETY: waits for one completion on the port this run owns.
        unsafe { GetQueuedCompletionStatus(port, &mut done, &mut key, &mut ov, 10_000) }
            .expect("completion");
        // SAFETY: every OVERLAPPED issued is the first field of a boxed `Slot` in `slots`.
        let slot = unsafe { &mut *(ov as *mut Slot) };
        max_ms = max_ms.max(monotonic_ms() - slot.issued);
        bytes += u64::from(done);
        ios += 1;
        in_flight -= 1;
        if issue(slot, &mut issued_bytes) {
            in_flight += 1;
        }
    }
    Run {
        bytes,
        ios,
        secs: (monotonic_ms() - started) / 1000.0,
        max_ms,
    }
}
