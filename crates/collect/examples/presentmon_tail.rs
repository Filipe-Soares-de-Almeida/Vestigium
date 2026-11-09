//! Runs PresentMon and reads its CSV: `presentmon_tail --exe <PresentMon.exe> --seconds <s> [--with-etw]`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

#[derive(Default)]
struct App {
    name: String,
    frames: u64,
    between_presents: Vec<f64>,
    between_display: Vec<f64>,
    not_displayed: u64,
    first_qpc: Option<i64>,
    last_qpc: Option<i64>,
    modes: HashMap<String, u64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exe = arg("--exe").ok_or("--exe <PresentMon.exe> is required")?;
    let seconds: u64 = arg("--seconds").and_then(|s| s.parse().ok()).unwrap_or(30);
    let etw = std::env::args().any(|a| a == "--with-etw");
    let freq = qpf();
    let mut kernel = None;
    if etw {
        let me = std::env::current_exe()?;
        let sibling = me.with_file_name("etw_dpc.exe");
        kernel = Some(
            Command::new(sibling)
                .args(["--seconds", &(seconds + 4).to_string()])
                .stdout(Stdio::piped())
                .spawn()?,
        );
    }
    let csv = arg("--csv");
    let output: Vec<String> = match &csv {
        Some(path) => vec!["--output_file".into(), path.clone()],
        None => vec!["--output_stdout".into()],
    };
    let mut extra: Vec<String> = Vec::new();
    if std::env::args().any(|a| a == "--no-display") {
        extra
            .extend(["--no_track_display", "--no_track_gpu", "--no_track_input"].map(String::from));
    }
    if let Some(name) = arg("--process") {
        extra.extend(["--process_name".to_string(), name]);
    }
    let start_qpc = qpc();
    let mut child = Command::new(&exe)
        .args(&output)
        .args(&extra)
        .args([
            "--set_circular_buffer_size",
            "8192",
            "--qpc_time",
            "--no_console_stats",
            "--timed",
            &seconds.to_string(),
            "--terminate_after_timed",
            "--session_name",
            "Vestigium PresentMon",
            "--stop_existing_session",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout: Box<dyn std::io::Read> = match &csv {
        Some(path) => {
            let _ = child.wait()?;
            Box::new(std::fs::File::open(path)?)
        }
        None => Box::new(child.stdout.take().ok_or("no stdout")?),
    };
    let mut header: Vec<String> = Vec::new();
    let mut apps: HashMap<(String, u32), App> = HashMap::new();
    let mut lines = 0u64;
    let mut bad = 0u64;
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        if header.is_empty() {
            header = line.split(',').map(|s| s.trim().to_string()).collect();
            println!("header: {}", header.join(","));
            continue;
        }
        lines += 1;
        let fields: Vec<&str> = line.split(',').collect();
        let get = |name: &str| -> Option<&str> {
            let at = header.iter().position(|h| h == name)?;
            fields.get(at).copied()
        };
        let (Some(name), Some(pid)) = (get("Application"), get("ProcessID")) else {
            bad += 1;
            continue;
        };
        let pid: u32 = pid.parse().unwrap_or(0);
        let app = apps.entry((name.to_string(), pid)).or_default();
        app.name = name.to_string();
        app.frames += 1;
        if let Some(v) = get("MsBetweenPresents").and_then(|v| v.parse().ok()) {
            app.between_presents.push(v);
        }
        match get("MsBetweenDisplayChange").and_then(|v| v.parse::<f64>().ok()) {
            Some(v) => app.between_display.push(v),
            None => app.not_displayed += 1,
        }
        if let Some(q) = get("CPUStartQPC").and_then(|v| v.parse::<i64>().ok()) {
            app.first_qpc.get_or_insert(q);
            app.last_qpc = Some(q);
        }
        if let Some(mode) = get("PresentMode") {
            *app.modes.entry(mode.to_string()).or_default() += 1;
        }
    }
    let status = child.wait()?;
    let end_qpc = qpc();
    let mut err = String::new();
    if let Some(mut e) = child.stderr.take() {
        use std::io::Read;
        let _ = e.read_to_string(&mut err);
    }
    println!("presentmon exit {status}; {lines} rows, {bad} unreadable");
    if !err.trim().is_empty() {
        println!("presentmon stderr: {}", err.trim());
    }
    let ms = |q: i64| (q - start_qpc) as f64 * 1000.0 / freq;
    println!("our QPC window: 0 .. {:.0} ms", ms(end_qpc));
    println!(
        "| app | pid | frames | not displayed | p50 between presents | p99 | max | p50 between display | p99 | hitches >2.5x median | first..last CPUStart (ms from our start) | modes |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
    let mut rows: Vec<_> = apps.into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.frames));
    for ((_, pid), app) in rows.iter().take(15) {
        let mut p = app.between_presents.clone();
        p.sort_by(f64::total_cmp);
        let mut d = app.between_display.clone();
        d.sort_by(f64::total_cmp);
        let med = pct(&p, 0.5);
        let hitches = app
            .between_presents
            .iter()
            .filter(|v| med > 0.0 && **v > 2.5 * med)
            .count();
        println!(
            "| {} | {pid} | {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {hitches} | {:.0}..{:.0} | {:?} |",
            app.name,
            app.frames,
            app.not_displayed,
            med,
            pct(&p, 0.99),
            pct(&p, 1.0),
            pct(&d, 0.5),
            pct(&d, 0.99),
            app.first_qpc.map_or(f64::NAN, ms),
            app.last_qpc.map_or(f64::NAN, ms),
            app.modes
        );
    }
    if let Some(k) = kernel {
        let out = k.wait_with_output()?;
        let text = String::from_utf8_lossy(&out.stdout);
        println!("\n--- etw_dpc alongside ---");
        for line in text.lines().take(14) {
            println!("{line}");
        }
    }
    Ok(())
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn qpc() -> i64 {
    let mut v = 0i64;
    // SAFETY: writes the counter into a local.
    let _ = unsafe { QueryPerformanceCounter(&mut v) };
    v
}

fn qpf() -> f64 {
    let mut v = 0i64;
    // SAFETY: writes the frequency into a local.
    let _ = unsafe { QueryPerformanceFrequency(&mut v) };
    v as f64
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1).cloned()
}
