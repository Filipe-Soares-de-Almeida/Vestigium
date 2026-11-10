use std::io::{BufRead, BufReader, Read};
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};

use crate::CollectError;

/// PresentMon's ETW session for the deep capture.
pub const PRESENTMON_SESSION: &str = "Vestigium PresentMon";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// How long past its own timer a PresentMon run may last before it is killed.
const DEADLINE_MARGIN_SECS: u64 = 15;

/// One presented frame: the app's own cadence, start time in raw QPC.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameRow {
    pub app: String,
    pub pid: u32,
    pub cpu_start_qpc: i64,
    pub between_presents_ms: Option<f32>,
}

/// How a PresentMon run ended.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresentMonOutcome {
    pub rows: u64,
    pub exit_code: Option<i32>,
    pub stderr: String,
    /// Seconds the run was asked for; 0 when it is not known.
    pub requested_secs: u64,
    /// Whole seconds the process ran.
    pub ran_secs: u64,
    /// The process outlived its deadline and was killed.
    pub timed_out: bool,
}

/// A run ends about a second after its timer; this much shorter than asked is an early end.
const EARLY_SLACK_SECS: u64 = 3;

impl PresentMonOutcome {
    fn ended_early(&self) -> bool {
        self.requested_secs > 0 && self.ran_secs + EARLY_SLACK_SECS < self.requested_secs
    }

    /// Why the frames cannot be trusted, if they cannot: presents seen but none finished, or no
    /// frame at all from a run that failed or stopped early.
    pub fn unavailable(&self) -> Option<String> {
        if self.rows > 0 {
            return None;
        }
        if self.stderr.contains("overflowed present events") {
            return Some("PresentMon saw presents but finished none".into());
        }
        if self.timed_out {
            return Some(format!(
                "PresentMon did not stop in time and was ended after {} s",
                self.ran_secs
            ));
        }
        if self.exit_code != Some(0) {
            return Some(format!(
                "PresentMon exited with {:?}: {}",
                self.exit_code,
                self.stderr.trim()
            ));
        }
        self.ended_early().then(|| {
            format!(
                "PresentMon stopped after {} of {} s",
                self.ran_secs, self.requested_secs
            )
        })
    }

    /// The seconds the frames cover and why that is less than asked, when frames were read but the
    /// run failed or stopped before its timer.
    pub fn partial(&self) -> Option<(u64, String)> {
        if self.rows == 0 || (self.exit_code == Some(0) && !self.ended_early() && !self.timed_out) {
            return None;
        }
        let covered = if self.requested_secs > 0 {
            self.ran_secs.min(self.requested_secs)
        } else {
            self.ran_secs
        };
        let how = match self.exit_code {
            _ if self.timed_out => "did not stop in time and was ended".to_string(),
            Some(0) => "stopped".to_string(),
            Some(code) => format!("exited with {code}"),
            None => "ended without an exit code".to_string(),
        };
        Some((
            covered,
            format!(
                "PresentMon {how} after {} of {} s",
                self.ran_secs, self.requested_secs
            ),
        ))
    }
}

/// Arguments for an app-cadence capture of `seconds`, without display, GPU or input tracking.
pub fn presentmon_args(seconds: u64) -> Vec<String> {
    [
        "--output_stdout",
        "--qpc_time",
        "--no_console_stats",
        "--no_track_display",
        "--no_track_gpu",
        "--no_track_input",
        "--set_circular_buffer_size",
        "8192",
        "--timed",
        &seconds.to_string(),
        "--terminate_after_timed",
        "--session_name",
        PRESENTMON_SESSION,
        "--stop_existing_session",
    ]
    .map(String::from)
    .to_vec()
}

/// Column positions in PresentMon's CSV, found by header name.
#[derive(Clone, Debug)]
pub struct CsvColumns {
    app: usize,
    pid: usize,
    cpu_start: usize,
    between: usize,
}

impl CsvColumns {
    pub fn new(header: &str) -> Option<CsvColumns> {
        let names: Vec<&str> = header.split(',').map(str::trim).collect();
        let at = |name: &str| names.iter().position(|n| *n == name);
        Some(CsvColumns {
            app: at("Application")?,
            pid: at("ProcessID")?,
            cpu_start: at("CPUStartQPC")?,
            between: at("MsBetweenPresents")?,
        })
    }

    /// The frame on one CSV line; `None` when a needed field is missing or unreadable.
    pub fn row(&self, line: &str) -> Option<FrameRow> {
        let fields: Vec<&str> = line.split(',').collect();
        Some(FrameRow {
            app: fields.get(self.app)?.trim().to_string(),
            pid: fields.get(self.pid)?.trim().parse().ok()?,
            cpu_start_qpc: fields.get(self.cpu_start)?.trim().parse().ok()?,
            between_presents_ms: fields.get(self.between)?.trim().parse().ok(),
        })
    }
}

/// Runs PresentMon at `exe` for `seconds` with no window, handing each frame to `on_frame`; a run
/// that outlives its timer by more than a margin is killed.
pub fn run_presentmon(
    exe: &Path,
    seconds: u64,
    on_frame: impl FnMut(FrameRow),
) -> Result<PresentMonOutcome, CollectError> {
    let deadline = Duration::from_secs(seconds + DEADLINE_MARGIN_SECS);
    run_with_args(exe, &presentmon_args(seconds), seconds, deadline, on_frame)
}

/// A job object that ends every process in it when its handle closes.
struct KillOnClose(HANDLE);

impl KillOnClose {
    /// A job holding `child`; `None` when Windows refuses one (the child then runs unguarded).
    fn holding(child: &Child) -> Option<KillOnClose> {
        // SAFETY: the job handle is owned by the returned value and closed once; the limit
        // structure outlives the call; the child's raw handle is valid while `child` lives.
        unsafe {
            let job = CreateJobObjectW(None, None).ok()?;
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ready = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&limits).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .is_ok()
                && AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())).is_ok();
            if ready {
                Some(KillOnClose(job))
            } else {
                let _ = CloseHandle(job);
                None
            }
        }
    }

    /// Ends every process in the job at once, children included.
    fn kill(&self) {
        // SAFETY: the handle is a live job object owned by `self`.
        unsafe {
            let _ = TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for KillOnClose {
    fn drop(&mut self) {
        // SAFETY: the handle is owned by `self` and closed once.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn run_with_args(
    exe: &Path,
    args: &[String],
    requested_secs: u64,
    deadline: Duration,
    mut on_frame: impl FnMut(FrameRow),
) -> Result<PresentMonOutcome, CollectError> {
    let started = Instant::now();
    let mut child = Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let job = KillOnClose::holding(&child);
    let mut stderr = child.stderr.take();
    let errors = thread::spawn(move || {
        let mut text = String::new();
        if let Some(pipe) = stderr.as_mut() {
            let _ = pipe.read_to_string(&mut text);
        }
        text
    });
    let (lines, incoming) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let failed = line.is_err();
                if lines.send(line).is_err() || failed {
                    break;
                }
            }
        });
    }
    let mut outcome = PresentMonOutcome {
        requested_secs,
        ..Default::default()
    };
    let mut columns = None;
    let mut read_error = None;
    loop {
        let left = deadline.saturating_sub(started.elapsed());
        match incoming.recv_timeout(left) {
            Ok(Ok(line)) => match &columns {
                None => columns = CsvColumns::new(&line),
                Some(cols) => {
                    if let Some(frame) = cols.row(&line) {
                        outcome.rows += 1;
                        on_frame(frame);
                    }
                }
            },
            Ok(Err(e)) => {
                read_error = Some(e);
                break;
            }
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                outcome.timed_out = true;
                break;
            }
        }
    }
    if outcome.timed_out || read_error.is_some() {
        match &job {
            Some(job) => job.kill(),
            None => {
                let _ = child.kill();
            }
        }
    }
    let status = child.wait();
    outcome.ran_secs = started.elapsed().as_secs();
    outcome.stderr = errors.join().unwrap_or_default();
    if let Some(e) = read_error {
        return Err(e.into());
    }
    outcome.exit_code = status?.code();
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,TimeInQPC,MsBetweenSimulationStart,MsBetweenPresents,MsInPresentAPI,MsRenderPresentLatency,CPUStartQPC,MsBetweenAppStart,MsCPUBusy,MsCPUWait";

    #[test]
    fn columns_by_header_name() {
        let cols = CsvColumns::new(HEADER).unwrap();
        let row = cols
            .row("game.exe,4242,0x1,DXGI,1,0,100,NA,16.7,0.2,1.0,123456789,16.6,3.0,13.6")
            .unwrap();
        assert_eq!(
            row,
            FrameRow {
                app: "game.exe".into(),
                pid: 4242,
                cpu_start_qpc: 123_456_789,
                between_presents_ms: Some(16.7)
            }
        );
        let shuffled =
            CsvColumns::new("CPUStartQPC,MsBetweenPresents,ProcessID,Application").unwrap();
        assert_eq!(shuffled.row("5,8.5,7,app.exe").unwrap().pid, 7);
    }

    #[test]
    fn na_between_presents_is_none() {
        let cols = CsvColumns::new(HEADER).unwrap();
        let row = cols
            .row("game.exe,4242,0x1,DXGI,1,0,100,NA,NA,0.2,1.0,123,NA,3.0,13.6")
            .unwrap();
        assert_eq!(row.between_presents_ms, None);
    }

    #[test]
    fn unknown_process_is_kept_as_unknown() {
        let cols = CsvColumns::new(HEADER).unwrap();
        let row = cols
            .row("<unknown>,99,0x1,DXGI,1,0,100,NA,5.0,0.2,1.0,123,NA,3.0,13.6")
            .unwrap();
        assert_eq!(row.app, "<unknown>");
    }

    #[test]
    fn header_without_the_needed_columns_is_refused() {
        assert!(CsvColumns::new("Application,ProcessID").is_none());
    }

    #[test]
    fn args_disable_display_tracking() {
        let args = presentmon_args(300);
        for flag in [
            "--no_track_display",
            "--no_track_gpu",
            "--no_track_input",
            "--qpc_time",
        ] {
            assert!(args.iter().any(|a| a == flag), "{flag}");
        }
        assert!(args.windows(2).any(|w| w[0] == "--timed" && w[1] == "300"));
    }

    #[test]
    fn empty_output_with_overflow_is_unavailable() {
        let outcome = PresentMonOutcome {
            rows: 0,
            exit_code: Some(0),
            stderr: "warning: 175 overflowed present events detected.".into(),
            ..Default::default()
        };
        assert!(outcome.unavailable().is_some());
        let quiet = PresentMonOutcome {
            rows: 0,
            exit_code: Some(0),
            requested_secs: 300,
            ran_secs: 301,
            ..Default::default()
        };
        assert_eq!(quiet.unavailable(), None);
        assert_eq!(quiet.partial(), None);
        let failed = PresentMonOutcome {
            rows: 0,
            exit_code: Some(5),
            stderr: "failed to start trace session (access denied)".into(),
            ..Default::default()
        };
        assert!(failed.unavailable().unwrap().contains("access denied"));
    }

    #[test]
    fn a_run_that_ends_before_its_timer_covers_less() {
        let whole = PresentMonOutcome {
            rows: 900,
            exit_code: Some(0),
            requested_secs: 300,
            ran_secs: 301,
            ..Default::default()
        };
        assert_eq!((whole.unavailable(), whole.partial()), (None, None));

        let died = PresentMonOutcome {
            rows: 900,
            exit_code: Some(0),
            requested_secs: 300,
            ran_secs: 180,
            ..Default::default()
        };
        assert_eq!(died.unavailable(), None);
        let (covered, why) = died.partial().expect("partial");
        assert_eq!(covered, 180);
        assert!(why.contains("180 of 300"), "{why}");

        let silent = PresentMonOutcome {
            rows: 0,
            exit_code: Some(0),
            requested_secs: 300,
            ran_secs: 20,
            ..Default::default()
        };
        assert!(silent.unavailable().unwrap().contains("20 of 300"));
        assert_eq!(silent.partial(), None);
    }

    #[test]
    fn presentmon_that_exits_early_with_rows_is_partial() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("rows.csv");
        std::fs::write(
            &csv,
            "Application,ProcessID,CPUStartQPC,MsBetweenPresents\r\ngame.exe,7,100,8.0\r\ngame.exe,7,200,9.0\r\n",
        )
        .unwrap();
        let args: Vec<String> = ["/D", "/C", "type", csv.to_str().unwrap(), "&", "exit", "1"]
            .map(String::from)
            .to_vec();

        let mut seen = 0;
        let outcome = run_with_args(
            Path::new("cmd.exe"),
            &args,
            300,
            Duration::from_secs(60),
            |_| seen += 1,
        )
        .unwrap();

        assert_eq!((seen, outcome.rows, outcome.exit_code), (2, 2, Some(1)));
        assert_eq!(outcome.unavailable(), None);
        let (covered, why) = outcome.partial().expect("a run that died is partial");
        assert!(covered < 300);
        assert!(why.contains("exited with 1"), "{why}");
    }

    fn ping_for(secs: u32) -> Vec<String> {
        ["-n", &secs.to_string(), "127.0.0.1"]
            .map(String::from)
            .to_vec()
    }

    #[test]
    fn presentmon_that_never_exits_is_killed_after_the_deadline() {
        let started = Instant::now();
        let outcome = run_with_args(
            Path::new("ping.exe"),
            &ping_for(40),
            300,
            Duration::from_secs(2),
            |_| {},
        )
        .unwrap();

        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(outcome.timed_out);
        assert!(outcome.unavailable().unwrap().contains("did not stop"));
    }

    #[test]
    fn a_run_that_was_ended_keeps_its_rows_as_partial() {
        let outcome = PresentMonOutcome {
            rows: 40,
            exit_code: Some(1),
            requested_secs: 300,
            ran_secs: 315,
            timed_out: true,
            ..Default::default()
        };
        assert_eq!(outcome.unavailable(), None);
        let (covered, why) = outcome.partial().expect("partial");
        assert_eq!(covered, 300);
        assert!(why.contains("did not stop in time"), "{why}");
    }

    #[test]
    fn closing_the_job_ends_the_process_in_it() {
        let mut child = Command::new("ping.exe")
            .args(ping_for(40))
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let job = KillOnClose::holding(&child).expect("a job object");
        drop(job);
        let started = Instant::now();
        child.wait().unwrap();
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
