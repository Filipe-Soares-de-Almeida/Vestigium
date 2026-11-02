use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use vestigium_collect::{
    DEEP_CAPTURE_GUID, DEEP_CAPTURE_SESSION, EventSink, KernelRecord, KernelSession, RawEvent,
    kernel_record, run_presentmon, spawn_consumer, wall_ms,
};
use vestigium_core::{
    AppFrames, CAPTURE_VERSION, CaptureSource, DeepCapture, DriverLatency, FrameAggregator,
    FrameHitch, LongRoutine, ModuleMap, RoutineAggregator,
};
use windows::Win32::System::Diagnostics::Etw::{
    EVENT_TRACE_FLAG_DPC, EVENT_TRACE_FLAG_IMAGE_LOAD, EVENT_TRACE_FLAG_INTERRUPT,
};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

use crate::{job, output, pinned};

/// Converts raw QPC values of this run to wall-clock milliseconds and microseconds.
#[derive(Clone, Copy, Debug)]
struct Clock {
    qpc0: i64,
    wall0: i64,
    freq: f64,
}

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
        }
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
            _ => {}
        }
    }
}

/// Runs the deep capture for `minutes` and writes it to `out`; `etl` also keeps the raw kernel trace.
pub fn run(minutes: u64, out: &Path, etl: Option<&Path>) -> ExitCode {
    if etl.is_some_and(|etl| output::check_shape(etl, "captures", "etl").is_err()) {
        return ExitCode::from(job::REFUSED);
    }
    let Ok(mut file) = output::create(out, "captures", "capture") else {
        return ExitCode::from(job::REFUSED);
    };
    let capture = capture(Duration::from_secs(minutes * 60), etl);
    let mut bytes = vec![CAPTURE_VERSION];
    match postcard::to_allocvec(&capture) {
        Ok(body) => bytes.extend(body),
        Err(_) => return ExitCode::from(job::WRITE_FAILED),
    }
    if file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        return ExitCode::from(job::WRITE_FAILED);
    }
    ExitCode::SUCCESS
}

fn capture(length: Duration, etl: Option<&Path>) -> DeepCapture {
    let clock = Clock::now();
    let flags = EVENT_TRACE_FLAG_DPC | EVENT_TRACE_FLAG_INTERRUPT | EVENT_TRACE_FLAG_IMAGE_LOAD;
    let session = KernelSession::start(DEEP_CAPTURE_SESSION, DEEP_CAPTURE_GUID, flags, etl);
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
    let frames = thread::spawn(move || frames(length, clock));
    thread::sleep(length);

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
                    Ok(sink) => (drivers, longest) = sink.routines.finish(&sink.modules),
                    Err(_) => kernel = CaptureSource::Unavailable("the trace reader failed".into()),
                },
                Some(Err(e)) => kernel = CaptureSource::Unavailable(e.to_string()),
                None => {}
            }
        }
    }
    let (frames, apps, hitches) = frames.join().unwrap_or_else(|_| {
        (
            CaptureSource::Unavailable("the frame reader failed".into()),
            vec![],
            vec![],
        )
    });
    DeepCapture {
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
    }
}

fn frames(length: Duration, clock: Clock) -> (CaptureSource, Vec<AppFrames>, Vec<FrameHitch>) {
    let dir = match std::env::current_exe() {
        Ok(exe) => exe.parent().map(Path::to_path_buf).unwrap_or_default(),
        Err(e) => return (CaptureSource::Unavailable(e.to_string()), vec![], vec![]),
    };
    let tool = match pinned::presentmon(&dir) {
        Ok(tool) => tool,
        Err(why) => return (CaptureSource::Unavailable(why), vec![], vec![]),
    };
    let mut aggregator = FrameAggregator::new();
    let outcome = run_presentmon(&tool.path, length.as_secs(), |row| {
        if let Some(ms) = row.between_presents_ms {
            aggregator.add(&row.app, row.pid, clock.wall_ms(row.cpu_start_qpc), ms);
        }
    });
    drop(tool);
    let source = match outcome {
        Ok(outcome) => outcome
            .unavailable()
            .map_or(CaptureSource::Ok, CaptureSource::Unavailable),
        Err(e) => CaptureSource::Unavailable(e.to_string()),
    };
    let (apps, hitches) = aggregator.finish();
    (source, apps, hitches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_converts_qpc_to_wall_and_micros() {
        let clock = Clock {
            qpc0: 1_000_000,
            wall0: 5_000,
            freq: 10_000_000.0,
        };
        assert_eq!(clock.wall_ms(1_000_000 + 25_000_000), 7_500);
        assert_eq!(clock.us(1_234), 123);
        assert_eq!(clock.wall_ms(990_000), 4_999);
    }
}
