use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::{
    DEEP_CAPTURE_GUID, DEEP_CAPTURE_LOCK, DEEP_CAPTURE_SESSION, EventSink, KernelRecord,
    KernelSession, PresentMonOutcome, RawEvent, kernel_record, run_presentmon, spawn_consumer,
    wall_ms,
};
use vestigium_core::{
    AppFrames, CAPTURE_VERSION, CaptureSource, DeepCapture, DriverLatency, FrameAggregator,
    FrameHitch, LongRoutine, ModuleMap, RoutineAggregator,
};
use windows::Win32::System::Diagnostics::Etw::{
    EVENT_TRACE_FLAG_DPC, EVENT_TRACE_FLAG_IMAGE_LOAD, EVENT_TRACE_FLAG_INTERRUPT,
};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

use crate::{job, lock, output, pinned};

/// Converts raw QPC values of this run to wall-clock milliseconds and microseconds.
#[derive(Clone, Copy, Debug)]
struct Clock {
    qpc0: i64,
    wall0: i64,
    freq: f64,
    /// Wall milliseconds that pass per QPC millisecond over the run, 1.0 until the run ends.
    scale: f64,
}

/// The counter and the wall clock may not drift apart by more than this share over one run.
const MAX_DRIFT: f64 = 0.01;
/// A run shorter than this, in milliseconds, is too short to measure a drift.
const MIN_RUN_MS: i64 = 1_000;

impl Clock {
    fn now() -> Clock {
        let (mut qpc, mut freq) = (0i64, 0i64);
        // SAFETY: both write into locals.
        unsafe {
            let _ = QueryPerformanceFrequency(&mut freq);
            let _ = QueryPerformanceCounter(&mut qpc);
        }
        Clock {
            qpc0: qpc,
            wall0: wall_ms(),
            freq: freq.max(1) as f64,
            scale: 1.0,
        }
    }

    /// Reads the counter and the wall clock again and sets the scale between them over the run.
    fn ended(mut self) -> Clock {
        let mut qpc = 0i64;
        // SAFETY: writes into a local.
        unsafe {
            let _ = QueryPerformanceCounter(&mut qpc);
        }
        self.scale = self.scale_to(qpc, wall_ms());
        self
    }

    /// The wall milliseconds per counter millisecond given the counter and wall clock at the end.
    fn scale_to(&self, qpc: i64, wall: i64) -> f64 {
        let counted = (qpc - self.qpc0) as f64 * 1000.0 / self.freq;
        let scale = (wall - self.wall0) as f64 / counted;
        if counted < MIN_RUN_MS as f64 || (scale - 1.0).abs() > MAX_DRIFT {
            1.0
        } else {
            scale
        }
    }

    /// A time converted with the start pair only, moved to where the scaled clock puts it.
    fn corrected(&self, ts_ms: i64) -> i64 {
        self.wall0 + ((ts_ms - self.wall0) as f64 * self.scale).round() as i64
    }

    fn wall_ms(&self, qpc: i64) -> i64 {
        self.wall0 + ((qpc - self.qpc0) as f64 * 1000.0 / self.freq).round() as i64
    }

    fn us(&self, ticks: i64) -> u32 {
        (ticks as f64 * 1e6 / self.freq)
            .round()
            .min(u32::MAX as f64) as u32
    }
}

struct Kernel {
    clock: Clock,
    routines: RoutineAggregator,
    modules: ModuleMap,
}

impl EventSink for Kernel {
    fn event(&mut self, event: &RawEvent<'_>) {
        match kernel_record(event) {
            Some(KernelRecord::Routine {
                kind,
                routine,
                initial,
            }) => {
                let ticks = event.timestamp - initial;
                if ticks >= 0 {
                    self.routines.add(
                        &self.modules,
                        routine,
                        kind,
                        self.clock.us(ticks),
                        event.cpu,
                        self.clock.wall_ms(event.timestamp),
                    );
                }
            }
            Some(KernelRecord::Image {
                base,
                size,
                pid: 0,
                path,
            }) => self.modules.insert(base, size, &path),
            Some(KernelRecord::ImageUnload { base, pid: 0 }) => self.modules.remove(base),
            _ => {}
        }
    }
}

/// How often a running capture looks for its stop file.
const STOP_POLL: Duration = Duration::from_millis(250);

/// Sleeps for `length`, looking at `stop` every `poll`; true when `stop` ended the wait early.
fn wait_out(length: Duration, poll: Duration, stop: &mut dyn FnMut() -> bool) -> bool {
    let started = Instant::now();
    loop {
        if stop() {
            return true;
        }
        let left = length.saturating_sub(started.elapsed());
        if left.is_zero() {
            return false;
        }
        thread::sleep(left.min(poll));
    }
}

/// Runs the deep capture for `minutes` and writes it to `out`, unless another capture is running;
/// a `.stop` file beside `out` ends it at once and keeps nothing.
pub fn run(minutes: u64, out: &Path) -> ExitCode {
    let Ok(_lock) = lock::take(DEEP_CAPTURE_LOCK) else {
        return ExitCode::from(job::CAPTURE_RUNNING);
    };
    let Ok(mut file) = output::create(out, "captures", "capture") else {
        return ExitCode::from(job::REFUSED);
    };
    let stop_path = out.with_extension("stop");
    let Some(capture) = capture(Duration::from_secs(minutes * 60), &mut || {
        stop_path.exists()
    }) else {
        return ExitCode::from(job::CANCELLED);
    };
    let mut bytes = vec![CAPTURE_VERSION];
    match postcard::to_allocvec(&capture) {
        Ok(body) => bytes.extend(body),
        Err(_) => return ExitCode::from(job::WRITE_FAILED),
    }
    if file.write_all(&bytes).is_err() || file.commit().is_err() {
        return ExitCode::from(job::WRITE_FAILED);
    }
    ExitCode::SUCCESS
}

/// The capture of `length`, or `None` when `stop` ended it early; the trace and PresentMon are
/// closed either way.
fn capture(length: Duration, stop: &mut dyn FnMut() -> bool) -> Option<DeepCapture> {
    let clock = Clock::now();
    let flags = EVENT_TRACE_FLAG_DPC | EVENT_TRACE_FLAG_INTERRUPT | EVENT_TRACE_FLAG_IMAGE_LOAD;
    let session = KernelSession::start(DEEP_CAPTURE_SESSION, DEEP_CAPTURE_GUID, flags);
    let consumer = session.as_ref().ok().map(|_| {
        spawn_consumer(
            DEEP_CAPTURE_SESSION,
            Kernel {
                clock,
                routines: RoutineAggregator::new(),
                modules: ModuleMap::new(),
            },
        )
    });
    let cancel = Arc::new(AtomicBool::new(false));
    let frames = {
        let cancel = cancel.clone();
        thread::spawn(move || frames(length, clock, &cancel))
    };
    let stopped = wait_out(length, STOP_POLL, stop);
    cancel.store(stopped, Ordering::Release);

    let mut kernel = CaptureSource::Ok;
    let mut lost_events = 0;
    let (mut drivers, mut longest): (Vec<DriverLatency>, Vec<LongRoutine>) = (vec![], vec![]);
    match session {
        Err(e) => kernel = CaptureSource::Unavailable(e.to_string()),
        Ok(session) => {
            match session.stop() {
                Ok(stats) => lost_events = stats.events_lost + stats.realtime_buffers_lost,
                Err(e) => kernel = CaptureSource::Unavailable(e.to_string()),
            }
            match consumer {
                Some(Ok(handle)) => match handle.join() {
                    Ok(read) => {
                        if let Some(why) = unread(read.events, &read.status) {
                            kernel = CaptureSource::Unavailable(why);
                        }
                        let sink = read.sink;
                        (drivers, longest) = sink.routines.finish();
                    }
                    Err(_) => kernel = CaptureSource::Unavailable("the trace reader failed".into()),
                },
                Some(Err(e)) => kernel = CaptureSource::Unavailable(e.to_string()),
                None => {}
            }
        }
    }
    let (frames, apps, mut hitches) = frames.join().unwrap_or_else(|_| {
        (
            CaptureSource::Unavailable("the frame reader failed".into()),
            vec![],
            vec![],
        )
    });
    if stopped {
        return None;
    }
    let clock = clock.ended();
    for routine in &mut longest {
        routine.ts_ms = clock.corrected(routine.ts_ms);
    }
    for hitch in &mut hitches {
        hitch.ts_ms = clock.corrected(hitch.ts_ms);
    }
    let mut capture = DeepCapture {
        start_ms: clock.wall0,
        end_ms: wall_ms(),
        kernel,
        frames,
        lost_events,
        drivers,
        longest,
        apps,
        hitches,
        hiccups: vec![],
    };
    capture.fit_to_ceiling();
    Some(capture)
}

fn frames(
    length: Duration,
    clock: Clock,
    cancel: &AtomicBool,
) -> (CaptureSource, Vec<AppFrames>, Vec<FrameHitch>) {
    let dir = match std::env::current_exe() {
        Ok(exe) => exe.parent().map(Path::to_path_buf).unwrap_or_default(),
        Err(e) => return (CaptureSource::Unavailable(e.to_string()), vec![], vec![]),
    };
    let tool = match pinned::presentmon(&dir) {
        Ok(tool) => tool,
        Err(why) => return (CaptureSource::Unavailable(why), vec![], vec![]),
    };
    let mut aggregator = FrameAggregator::new();
    let outcome = run_presentmon(&tool.path, length.as_secs(), cancel, |row| {
        if let Some(ms) = row.between_presents_ms {
            aggregator.add(&row.app, row.pid, clock.wall_ms(row.cpu_start_qpc), ms);
        }
    });
    drop(tool);
    let source = match outcome {
        Ok(outcome) => frames_source(&outcome),
        Err(e) => CaptureSource::Unavailable(e.to_string()),
    };
    let (apps, hitches) = aggregator.finish();
    (source, apps, hitches)
}

/// Why a trace reader's work cannot be trusted: its session failed, or it received no event at all.
fn unread(events: u64, status: &Result<(), String>) -> Option<String> {
    match status {
        Err(why) => Some(why.clone()),
        Ok(()) if events == 0 => Some("the trace reader received no events".into()),
        Ok(()) => None,
    }
}

/// What a finished PresentMon run says about the frames: whole, partial or not measured.
fn frames_source(outcome: &PresentMonOutcome) -> CaptureSource {
    if let Some(why) = outcome.unavailable() {
        CaptureSource::Unavailable(why)
    } else if let Some((covered_secs, reason)) = outcome.partial() {
        CaptureSource::Partial {
            covered_secs,
            reason,
        }
    } else {
        CaptureSource::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stop_ends_the_wait_before_the_length() {
        let started = Instant::now();
        let mut looks = 0;
        let stopped = wait_out(
            Duration::from_secs(60),
            Duration::from_millis(10),
            &mut || {
                looks += 1;
                looks > 3
            },
        );
        assert!(stopped);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_wait_nobody_stops_lasts_the_length() {
        let started = Instant::now();
        let stopped = wait_out(
            Duration::from_millis(120),
            Duration::from_millis(10),
            &mut || false,
        );
        assert!(!stopped);
        assert!(started.elapsed() >= Duration::from_millis(120));
    }

    #[test]
    fn a_reader_that_failed_or_saw_nothing_is_not_trusted() {
        assert_eq!(unread(10, &Ok(())), None);
        assert!(unread(0, &Ok(())).is_some());
        assert_eq!(unread(10, &Err("closed".into())).as_deref(), Some("closed"));
        assert!(unread(0, &Err("not opened".into())).is_some());
    }

    #[test]
    fn a_run_that_died_is_a_partial_source() {
        let died = PresentMonOutcome {
            rows: 10,
            exit_code: Some(1),
            requested_secs: 300,
            ran_secs: 180,
            ..Default::default()
        };
        assert!(matches!(
            frames_source(&died),
            CaptureSource::Partial {
                covered_secs: 180,
                ..
            }
        ));
        let whole = PresentMonOutcome {
            rows: 10,
            exit_code: Some(0),
            requested_secs: 300,
            ran_secs: 301,
            ..Default::default()
        };
        assert_eq!(frames_source(&whole), CaptureSource::Ok);
        let none = PresentMonOutcome {
            exit_code: Some(1),
            ..Default::default()
        };
        assert!(matches!(
            frames_source(&none),
            CaptureSource::Unavailable(_)
        ));
    }

    #[test]
    fn clock_converts_qpc_to_wall_and_micros() {
        let clock = Clock {
            qpc0: 1_000_000,
            wall0: 5_000,
            freq: 10_000_000.0,
            scale: 1.0,
        };
        assert_eq!(clock.wall_ms(1_000_000 + 25_000_000), 7_500);
        assert_eq!(clock.us(1_234), 123);
        assert_eq!(clock.wall_ms(990_000), 4_999);
    }

    #[test]
    fn times_follow_the_wall_clock_over_the_run() {
        let mut clock = Clock {
            qpc0: 0,
            wall0: 10_000,
            freq: 1_000.0,
            scale: 1.0,
        };
        // 3,600 s counted, 3,600.18 s on the wall: the counter runs 50 ppm slow.
        let scale = clock.scale_to(3_600_000, 10_000 + 3_600_180);
        assert!((scale - 1.00005).abs() < 1e-9);
        clock.scale = scale;
        assert_eq!(clock.corrected(10_000), 10_000);
        assert_eq!(clock.corrected(10_000 + 3_600_000), 10_000 + 3_600_180);
        // A wall clock that jumped, or a run too short to tell, leaves times as counted.
        assert_eq!(clock.scale_to(3_600_000, 10_000 + 3_000_000), 1.0);
        assert_eq!(clock.scale_to(500, 10_000 + 900), 1.0);
    }
}
