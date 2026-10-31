//! Measures per-process GPU counters:
//! `gpu_engine_cost --seconds <s> --read-every <s> --expand-every <s>`.

use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

use argos_collect::{logical_cpus, monotonic_ms};
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhExpandWildCardPathW,
    PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::core::{HSTRING, PCWSTR, PWSTR};

const PATH: &str = r"\GPU Engine(*)\Utilization Percentage";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds = arg("--seconds").unwrap_or(60);
    let read_every = arg("--read-every").unwrap_or(5).max(1);
    let expand_every = arg("--expand-every").unwrap_or(60).max(1);

    let mut query = PDH_HQUERY::default();
    let mut counter = PDH_HCOUNTER::default();
    // SAFETY: opens a live query and adds one wildcard counter to it; both closed at the end.
    unsafe {
        status(PdhOpenQueryW(PCWSTR::null(), 0, &mut query))?;
        status(PdhAddEnglishCounterW(
            query,
            &HSTRING::from(PATH),
            0,
            &mut counter,
        ))?;
        status(PdhCollectQueryData(query))?;
    }

    let cpu_before = process_cpu_ms();
    let mut buf: Vec<u64> = Vec::new();
    let (mut collect_us, mut read_us, mut expand_us) = (Vec::new(), Vec::new(), Vec::new());
    let (mut instances, mut pids) = (Vec::new(), Vec::new());
    let mut elapsed = 0;
    while elapsed < seconds {
        thread::sleep(Duration::from_secs(read_every));
        elapsed += read_every;

        let t0 = monotonic_ms();
        // SAFETY: the query is open.
        status(unsafe { PdhCollectQueryData(query) })?;
        let t1 = monotonic_ms();
        let values = read(counter, &mut buf)?;
        let t2 = monotonic_ms();
        collect_us.push((t1 - t0) * 1000.0);
        read_us.push((t2 - t1) * 1000.0);
        instances.push(values.len() as f64);
        let per_pid: HashMap<u32, f64> =
            values.iter().fold(HashMap::new(), |mut acc, (name, v)| {
                if let Some(pid) = pid_of(name) {
                    *acc.entry(pid).or_default() += v;
                }
                acc
            });
        pids.push(per_pid.keys().collect::<HashSet<_>>().len() as f64);

        if elapsed % expand_every < read_every {
            let t = monotonic_ms();
            expand()?;
            expand_us.push((monotonic_ms() - t) * 1000.0);
        }
    }
    let cpu_ms = process_cpu_ms() - cpu_before;
    // SAFETY: closes the query opened above.
    let _ = unsafe { PdhCloseQuery(query) };

    let machine_pct = cpu_ms / (seconds as f64 * 1000.0 * f64::from(logical_cpus())) * 100.0;
    println!(
        "| reads | instances avg | pids avg | collect us p50/max | read us p50/max | expand us avg | CPU ms | CPU % machine |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    println!(
        "| {} | {:.0} | {:.0} | {:.0} / {:.0} | {:.0} / {:.0} | {:.0} | {cpu_ms:.0} | {machine_pct:.4} |",
        collect_us.len(),
        mean(&instances),
        mean(&pids),
        median(&mut collect_us),
        max(&collect_us),
        median(&mut read_us),
        max(&read_us),
        mean(&expand_us),
    );
    Ok(())
}

fn read(counter: PDH_HCOUNTER, buf: &mut Vec<u64>) -> Result<Vec<(String, f64)>, String> {
    loop {
        let mut bytes = (buf.len() * 8) as u32;
        let mut count = 0u32;
        let target =
            (!buf.is_empty()).then(|| buf.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>());
        // SAFETY: `target`, when given, has `bytes` writable bytes aligned to 8.
        let code = unsafe {
            PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, target)
        };
        if code == PDH_MORE_DATA || (code == 0 && target.is_none() && bytes > 0) {
            buf.resize((bytes as usize).div_ceil(8), 0);
            continue;
        }
        status(code)?;
        // SAFETY: on success the buffer starts with `count` items naming strings inside it.
        let items = unsafe {
            std::slice::from_raw_parts(
                buf.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                count as usize,
            )
        };
        return Ok(items
            .iter()
            .filter(|item| item.FmtValue.CStatus <= 1)
            // SAFETY: valid items hold a NUL-terminated name and a double.
            .map(|item| unsafe {
                (
                    String::from_utf16_lossy(item.szName.as_wide()),
                    item.FmtValue.Anonymous.doubleValue,
                )
            })
            .collect());
    }
}

fn expand() -> Result<(), String> {
    let path = HSTRING::from(PATH);
    let mut len = 0u32;
    // SAFETY: a size query, then a fill into a buffer of exactly that many UTF-16 units.
    unsafe {
        let _ = PdhExpandWildCardPathW(PCWSTR::null(), &path, None, &mut len, 0);
        let mut list = vec![0u16; len as usize];
        status(PdhExpandWildCardPathW(
            PCWSTR::null(),
            &path,
            Some(PWSTR(list.as_mut_ptr())),
            &mut len,
            0,
        ))
    }
}

/// "pid_1234_luid_0x0_0x1_phys_0_eng_0_engtype_3D" → 1234.
fn pid_of(instance: &str) -> Option<u32> {
    instance
        .strip_prefix("pid_")?
        .split('_')
        .next()?
        .parse()
        .ok()
}

fn status(code: u32) -> Result<(), String> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!("PDH status {code:#010x}"))
    }
}

fn arg(name: &str) -> Option<u64> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1)?.parse().ok()
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

fn max(values: &[f64]) -> f64 {
    values.iter().copied().fold(0.0, f64::max)
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
