use std::io::{BufRead, BufReader, Read};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

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
}

impl PresentMonOutcome {
    /// Why the frames cannot be trusted, if they cannot: presents seen but none finished.
    pub fn unavailable(&self) -> Option<String> {
        if self.rows == 0 && self.stderr.contains("overflowed present events") {
            return Some("PresentMon saw presents but finished none".into());
        }
        if self.exit_code != Some(0) && self.rows == 0 {
            return Some(format!(
                "PresentMon exited with {:?}: {}",
                self.exit_code,
                self.stderr.trim()
            ));
        }
        None
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
    mut on_frame: impl FnMut(FrameRow),
) -> Result<PresentMonOutcome, CollectError> {
    let mut child = Command::new(exe)
        .args(presentmon_args(seconds))
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
    let mut outcome = PresentMonOutcome::default();
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
        };
        assert!(outcome.unavailable().is_some());
        let quiet = PresentMonOutcome {
            rows: 0,
            exit_code: Some(0),
            stderr: String::new(),
        };
        assert_eq!(quiet.unavailable(), None);
        let failed = PresentMonOutcome {
            rows: 0,
            exit_code: Some(5),
            stderr: "failed to start trace session (access denied)".into(),
        };
        assert!(failed.unavailable().unwrap().contains("access denied"));
    }
}
