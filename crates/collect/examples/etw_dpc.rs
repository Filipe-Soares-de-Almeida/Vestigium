//! Traces DPC, ISR and image events in an own system session: `etw_dpc --seconds <s> [--etl <path>]`.

use std::collections::HashMap;
use std::mem::offset_of;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, ERROR_SUCCESS, FILETIME, WIN32_ERROR};
use windows::Win32::System::Diagnostics::Etw::{
    CONTROLTRACE_HANDLE, CloseTrace, ControlTraceW, EVENT_RECORD, EVENT_TRACE_CONTROL,
    EVENT_TRACE_CONTROL_QUERY, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_FILE_MODE_SEQUENTIAL,
    EVENT_TRACE_FLAG_DPC, EVENT_TRACE_FLAG_IMAGE_LOAD, EVENT_TRACE_FLAG_INTERRUPT,
    EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE,
    EVENT_TRACE_SYSTEM_LOGGER_MODE, OpenTraceW, PROCESS_TRACE_MODE_EVENT_RECORD,
    PROCESS_TRACE_MODE_RAW_TIMESTAMP, PROCESS_TRACE_MODE_REAL_TIME, ProcessTrace, StartTraceW,
    WNODE_FLAG_TRACED_GUID,
};
use windows::Win32::System::Performance::QueryPerformanceFrequency;
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::core::{GUID, PCWSTR, PWSTR};

const SESSION: &str = "Vestigium Deep Capture";
const SESSION_GUID: GUID = GUID::from_u128(0x5e0c_7a1d_4b2f_4c8e_9a63_d1f2_7b4e_08c5);
const PERF_INFO: GUID = GUID::from_u128(0xce1dbfb4_137e_4da6_87b0_3f59aa102cbc);
const IMAGE: GUID = GUID::from_u128(0x2cb15d1d_5fc1_11d2_abe1_00a0c911f518);
/// Upper bounds of the tracerpt dpcisr report buckets, in microseconds.
const BOUNDS_US: [f64; 12] = [
    1.0, 2.0, 3.0, 4.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
];

#[repr(C)]
struct Properties {
    props: EVENT_TRACE_PROPERTIES,
    name: [u16; 64],
    file: [u16; 260],
}

#[derive(Clone, Copy, Default)]
struct Stats {
    count: u64,
    total_ticks: i64,
    max_ticks: i64,
    buckets: [u64; 13],
}

#[derive(Default)]
struct Trace {
    freq: f64,
    events: u64,
    negative: u64,
    dpc: HashMap<u64, Stats>,
    isr: HashMap<u64, Stats>,
    per_cpu_ticks: HashMap<(u8, bool), i64>,
    modules: Vec<(u64, u64, String)>,
    opcodes: HashMap<(u8, u8), u64>,
}

static TRACE: Mutex<Option<Trace>> = Mutex::new(None);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds = arg("--seconds")
        .and_then(|s| s.parse().ok())
        .unwrap_or(60u64);
    let etl = arg("--etl");
    let mut freq = 0i64;
    // SAFETY: writes the counter frequency into a local.
    unsafe { QueryPerformanceFrequency(&mut freq)? };
    *TRACE.lock().unwrap() = Some(Trace {
        freq: freq as f64,
        ..Trace::default()
    });

    let mut props = properties(etl.as_deref());
    let name = wide(SESSION);
    let mut handle = CONTROLTRACE_HANDLE::default();
    // SAFETY: props is a zeroed, sized EVENT_TRACE_PROPERTIES followed by the name and file buffers.
    let mut status = unsafe { StartTraceW(&mut handle, PCWSTR(name.as_ptr()), &mut props.props) };
    if status == ERROR_ALREADY_EXISTS {
        println!("orphan session found; stopping it");
        let mut stale = properties(None);
        let _ = control(
            CONTROLTRACE_HANDLE::default(),
            &mut stale,
            EVENT_TRACE_CONTROL_STOP,
        );
        props = properties(etl.as_deref());
        // SAFETY: as above, with fresh properties.
        status = unsafe { StartTraceW(&mut handle, PCWSTR(name.as_ptr()), &mut props.props) };
    }
    if status != ERROR_SUCCESS {
        return Err(format!("StartTraceW failed: {}", status.0).into());
    }
    let cpu_before = process_cpu_ms();
    let mut kernel_counters = vestigium_collect::CpuSampler::new();
    let _ = kernel_counters.sample();
    let started = Instant::now();

    let consumer = thread::spawn(move || {
        let mut logger = wide(SESSION);
        let mut logfile = EVENT_TRACE_LOGFILEW {
            LoggerName: PWSTR(logger.as_mut_ptr()),
            ..Default::default()
        };
        logfile.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME
            | PROCESS_TRACE_MODE_EVENT_RECORD
            | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
        logfile.Anonymous2.EventRecordCallback = Some(on_event);
        // SAFETY: logfile and the logger name outlive the trace, which ends before this closure returns.
        let trace = unsafe { OpenTraceW(&mut logfile) };
        if trace.Value == u64::MAX {
            eprintln!("OpenTraceW failed");
            return;
        }
        // SAFETY: trace is the handle OpenTraceW just returned.
        let status = unsafe { ProcessTrace(&[trace], None, None) };
        println!("ProcessTrace returned {}", status.0);
        // SAFETY: closes the handle opened above.
        let _ = unsafe { CloseTrace(trace) };
    });

    thread::sleep(Duration::from_secs(seconds));
    let counters = kernel_counters.sample().ok().flatten();
    let mut query = properties(None);
    let _ = control(handle, &mut query, EVENT_TRACE_CONTROL_QUERY);
    let mut stop = properties(None);
    let stopped = control(handle, &mut stop, EVENT_TRACE_CONTROL_STOP);
    consumer.join().map_err(|_| "consumer panicked")?;
    let elapsed = started.elapsed().as_secs_f64();
    let cpu_ms = process_cpu_ms() - cpu_before;

    println!("stop status {}", stopped.0);
    println!(
        "buffers {} free {} written {} events lost {} log buffers lost {} realtime buffers lost {}",
        stop.props.NumberOfBuffers,
        stop.props.FreeBuffers,
        stop.props.BuffersWritten,
        stop.props.EventsLost,
        stop.props.LogBuffersLost,
        stop.props.RealTimeBuffersLost
    );
    report(elapsed, cpu_ms, counters);
    Ok(())
}

fn properties(etl: Option<&str>) -> Box<Properties> {
    // SAFETY: Properties is plain data; all-zero is its documented starting state.
    let mut p: Box<Properties> = Box::new(unsafe { std::mem::zeroed() });
    p.props.Wnode.BufferSize = size_of::<Properties>() as u32;
    p.props.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
    p.props.Wnode.Guid = SESSION_GUID;
    p.props.Wnode.ClientContext = 1;
    p.props.BufferSize = 64;
    p.props.MinimumBuffers = 64;
    p.props.MaximumBuffers = 128;
    p.props.FlushTimer = 1;
    p.props.LogFileMode = EVENT_TRACE_SYSTEM_LOGGER_MODE | EVENT_TRACE_REAL_TIME_MODE;
    p.props.EnableFlags =
        EVENT_TRACE_FLAG_DPC | EVENT_TRACE_FLAG_INTERRUPT | EVENT_TRACE_FLAG_IMAGE_LOAD;
    p.props.LoggerNameOffset = offset_of!(Properties, name) as u32;
    if let Some(path) = etl {
        p.props.LogFileMode |= EVENT_TRACE_FILE_MODE_SEQUENTIAL;
        p.props.LogFileNameOffset = offset_of!(Properties, file) as u32;
        for (slot, unit) in p
            .file
            .iter_mut()
            .zip(path.replace('/', "\\").encode_utf16())
        {
            *slot = unit;
        }
    }
    p
}

fn control(
    handle: CONTROLTRACE_HANDLE,
    props: &mut Properties,
    code: EVENT_TRACE_CONTROL,
) -> WIN32_ERROR {
    let name = wide(SESSION);
    // SAFETY: props is sized for the name and file buffers that follow it.
    unsafe { ControlTraceW(handle, PCWSTR(name.as_ptr()), &mut props.props, code) }
}

unsafe extern "system" fn on_event(record: *mut EVENT_RECORD) {
    // SAFETY: ETW passes a valid record for the duration of the callback.
    let record = unsafe { &*record };
    let header = &record.EventHeader;
    let data: &[u8] = if record.UserData.is_null() {
        &[]
    } else {
        // SAFETY: UserData points to UserDataLength bytes owned by ETW during the callback.
        unsafe {
            std::slice::from_raw_parts(record.UserData as *const u8, record.UserDataLength.into())
        }
    };
    // SAFETY: the processor number is the documented member of the buffer context union.
    let cpu = unsafe { record.BufferContext.Anonymous.Anonymous.ProcessorNumber };
    let opcode = header.EventDescriptor.Opcode;
    let mut guard = TRACE.lock().unwrap();
    let Some(trace) = guard.as_mut() else { return };
    trace.events += 1;
    if header.ProviderId == PERF_INFO && matches!(opcode, 50 | 66..=69) && data.len() >= 16 {
        let initial = i64::from_le_bytes(data[0..8].try_into().unwrap());
        let routine = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let ticks = header.TimeStamp - initial;
        if ticks < 0 {
            trace.negative += 1;
            return;
        }
        let us = ticks as f64 * 1e6 / trace.freq;
        let bucket = BOUNDS_US.iter().position(|b| us < *b).unwrap_or(12);
        let map = if matches!(opcode, 50 | 67) {
            &mut trace.isr
        } else {
            &mut trace.dpc
        };
        let s = map.entry(routine).or_default();
        s.count += 1;
        s.total_ticks += ticks;
        s.max_ticks = s.max_ticks.max(ticks);
        s.buckets[bucket] += 1;
        *trace
            .per_cpu_ticks
            .entry((cpu, matches!(opcode, 50 | 67)))
            .or_default() += ticks;
    } else if header.ProviderId == IMAGE && matches!(opcode, 3 | 10) && data.len() > 58 {
        let base = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let size = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let pid = u32::from_le_bytes(data[16..20].try_into().unwrap());
        if pid == 0 {
            let units: Vec<u16> = data[56..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .take_while(|u| *u != 0)
                .collect();
            let path = String::from_utf16_lossy(&units);
            let name = path.rsplit('\\').next().unwrap_or(&path).to_string();
            trace.modules.push((base, size, name));
        }
    }
    *trace
        .opcodes
        .entry((header.EventDescriptor.Version, opcode))
        .or_default() += 1;
}

fn report(elapsed: f64, cpu_ms: f64, counters: Option<vestigium_collect::CpuReading>) {
    let guard = TRACE.lock().unwrap();
    let trace = guard.as_ref().unwrap();
    let mut modules = trace.modules.clone();
    modules.sort_by_key(|m| m.0);
    modules.dedup_by_key(|m| m.0);
    let find = |addr: u64| -> Option<&str> {
        let at = modules.partition_point(|m| m.0 <= addr);
        let m = modules.get(at.checked_sub(1)?)?;
        (addr < m.0 + m.1).then_some(m.2.as_str())
    };
    let us = |t: i64| t as f64 * 1e6 / trace.freq;
    println!(
        "elapsed {elapsed:.1} s, events {} ({:.0}/s), negative durations {}, kernel modules {}, consumer CPU {cpu_ms:.0} ms ({:.3}% of one core)",
        trace.events,
        trace.events as f64 / elapsed,
        trace.negative,
        modules.len(),
        cpu_ms / (elapsed * 10.0)
    );
    for (label, map) in [("DPC", &trace.dpc), ("ISR", &trace.isr)] {
        let mut per: HashMap<&str, Stats> = HashMap::new();
        let mut unmapped = 0u64;
        for (routine, s) in map {
            let name = find(*routine).unwrap_or_else(|| {
                unmapped += s.count;
                "<unmapped>"
            });
            let e = per.entry(name).or_default();
            e.count += s.count;
            e.total_ticks += s.total_ticks;
            e.max_ticks = e.max_ticks.max(s.max_ticks);
            for (a, b) in e.buckets.iter_mut().zip(s.buckets) {
                *a += b;
            }
        }
        let total: u64 = per.values().map(|s| s.count).sum();
        println!("\n## {label}: {total} calls, {unmapped} unmapped");
        println!(
            "| module | count | total ms | % of one core | max us | >=100us | >=1ms | buckets <1,<2,<3,<4,<5,<10,<25,<50,<100,<250,<500,<1000,>=1000 |"
        );
        println!("|---|---|---|---|---|---|---|---|");
        let mut rows: Vec<_> = per.into_iter().collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.1.total_ticks));
        for (name, s) in rows.iter().take(25) {
            let over100: u64 = s.buckets[9..].iter().sum();
            println!(
                "| {name} | {} | {:.2} | {:.3} | {:.0} | {over100} | {} | {:?} |",
                s.count,
                us(s.total_ticks) / 1000.0,
                us(s.total_ticks) / (elapsed * 1e4),
                us(s.max_ticks),
                s.buckets[12],
                s.buckets
            );
        }
    }
    println!("\n## Time per CPU: ETW vs the kernel's own counters");
    println!("| cpu | ETW DPC % | kernel DPC % | ETW ISR % | kernel ISR % |");
    println!("|---|---|---|---|---|");
    let cores = counters.map(|c| c.cores).unwrap_or_default();
    for cpu in 0..cores.len().max(8) as u8 {
        let pct =
            |isr: bool| us(*trace.per_cpu_ticks.get(&(cpu, isr)).unwrap_or(&0)) / (elapsed * 1e4);
        let k = cores.get(cpu as usize);
        println!(
            "| {cpu} | {:.3} | {:.3} | {:.3} | {:.3} |",
            pct(false),
            k.map_or(f32::NAN, |c| c.dpc_pct),
            pct(true),
            k.map_or(f32::NAN, |c| c.isr_pct)
        );
    }
    let mut ops: Vec<_> = trace.opcodes.iter().collect();
    ops.sort();
    println!("\n(version, opcode) counts: {ops:?}");
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1).cloned()
}

fn process_cpu_ms() -> f64 {
    let (mut created, mut exited, mut kernel, mut user) = Default::default();
    // SAFETY: queries the current process into four local FILETIMEs.
    let _ = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    };
    (filetime_100ns(kernel) + filetime_100ns(user)) as f64 / 10_000.0
}

fn filetime_100ns(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}
