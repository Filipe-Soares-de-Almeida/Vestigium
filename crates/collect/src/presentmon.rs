use std::io::{BufRead, BufReader, Read};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Instant;

use crate::CollectError;

/// PresentMon's ETW session for the deep capture.
pub const PRESENTMON_SESSION: &str = "Vestigium PresentMon";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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
        if self.rows == 0 || (self.exit_code == Some(0) && !self.ended_early()) {
            return None;
        }
        let covered = if self.requested_secs > 0 {
            self.ran_secs.min(self.requested_secs)
        } else {
            self.ran_secs
        };
        let how = match self.exit_code {
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

/// Runs PresentMon at `exe` for `seconds` with no window, handing each frame to `on_frame`.
pub fn run_presentmon(
    exe: &Path,
    seconds: u64,
    on_frame: impl FnMut(FrameRow),
) -> Result<PresentMonOutcome, CollectError> {
    run_with_args(exe, &presentmon_args(seconds), seconds, on_frame)
}

fn run_with_args(
    exe: &Path,
    args: &[String],
    requested_secs: u64,
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
    let mut stderr = child.stderr.take();
    let errors = thread::spawn(move || {
        let mut text = String::new();
        if let Some(pipe) = stderr.as_mut() {
            let _ = pipe.read_to_string(&mut text);
        }
        text
    });
    let mut outcome = PresentMonOutcome {
        requested_secs,
        ..Default::default()
    };
    if let Some(stdout) = child.stdout.take() {
        let mut columns = None;
        for line in BufReader::new(stdout).lines() {
            let line = line?;
            match &columns {
                None => columns = CsvColumns::new(&line),
                Some(cols) => {
                    if let Some(frame) = cols.row(&line) {
                        outcome.rows += 1;
                        on_frame(frame);
                    }
                }
            }
        }
    }
    outcome.exit_code = child.wait()?.code();
    outcome.ran_secs = started.elapsed().as_secs();
    outcome.stderr = errors.join().unwrap_or_default();
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
        let outcome = run_with_args(Path::new("cmd.exe"), &args, 300, |_| seen += 1).unwrap();

        assert_eq!((seen, outcome.rows, outcome.exit_code), (2, 2, Some(1)));
        assert_eq!(outcome.unavailable(), None);
        let (covered, why) = outcome.partial().expect("a run that died is partial");
        assert!(covered < 300);
        assert!(why.contains("exited with 1"), "{why}");
    }
}
