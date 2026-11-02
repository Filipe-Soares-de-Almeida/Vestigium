//! Induces the freezes and pressure Vestigium must detect: `vestigium-scenarios <scenario> [options]`.

mod cpu;
mod disk;
mod frame;
mod hang;
mod memory;
mod starve;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::core::BOOL;

/// Set by Ctrl+C; every scenario checks it and winds down.
static STOP: AtomicBool = AtomicBool::new(false);

/// What to induce, with its options already capped.
#[derive(Clone, Debug, PartialEq)]
enum Scenario {
    CpuBurn {
        seconds: u64,
    },
    MemoryPressure {
        hold_secs: u64,
        avail_pct: f64,
    },
    DiskSaturate {
        file: PathBuf,
        seconds: u64,
        threads: usize,
        create_gb: Option<u64>,
    },
    IoBurst {
        file: PathBuf,
        seconds: u64,
        create_gb: Option<u64>,
    },
    HangWindow {
        seconds: u64,
    },
    Starve {
        seconds: u64,
    },
    FrameHitch {
        seconds: u64,
        every_secs: u64,
        hitch_ms: u64,
    },
}

impl Scenario {
    /// What Vestigium should report while or after this runs.
    fn expected(&self) -> &'static str {
        match self {
            Scenario::CpuBurn { .. } => {
                "a \"CPU saturated\" episode of at least 60 s, with vestigium-scenarios.exe as the culprit"
            }
            Scenario::MemoryPressure { .. } => {
                "a \"memory high\" episode (over 90% in use for 2 min or more), with vestigium-scenarios.exe as the culprit"
            }
            Scenario::DiskSaturate { .. } => {
                "a \"disk busy\" episode on the disk holding the file, with vestigium-scenarios.exe as the culprit"
            }
            Scenario::IoBurst { .. } => {
                "a disk latency hint in default mode, or nothing if the disk absorbs the bursts"
            }
            Scenario::HangWindow { .. } => {
                "an \"app not responding\" occurrence naming vestigium-scenarios.exe, with high confidence"
            }
            Scenario::Starve { .. } => {
                "a freeze occurrence of 1-5 s, with CPU as the top suspect and vestigium-scenarios.exe first"
            }
            Scenario::FrameHitch { .. } => {
                "in a deep capture, vestigium-scenarios.exe with one hitched frame at each stall"
            }
        }
    }

    fn run(self) -> Result<(), String> {
        match self {
            Scenario::CpuBurn { seconds } => cpu::burn(seconds),
            Scenario::MemoryPressure {
                hold_secs,
                avail_pct,
            } => memory::pressure(hold_secs, avail_pct),
            Scenario::DiskSaturate {
                file,
                seconds,
                threads,
                create_gb,
            } => {
                let len = disk::prepare(&file, create_gb)?;
                disk::read_load(&file, len, seconds, threads, None)
            }
            Scenario::IoBurst {
                file,
                seconds,
                create_gb,
            } => {
                let len = disk::prepare(&file, create_gb)?;
                disk::read_load(&file, len, seconds, 8, Some(disk::BURST))
            }
            Scenario::HangWindow { seconds } => hang::hang(seconds),
            Scenario::Starve { seconds } => starve::starve(seconds),
            Scenario::FrameHitch {
                seconds,
                every_secs,
                hitch_ms,
            } => frame::hitch(seconds, every_secs, hitch_ms),
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let scenario = match parse(&args) {
        Ok(scenario) => scenario,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    // SAFETY: the handler only sets an atomic flag.
    if unsafe { SetConsoleCtrlHandler(Some(on_ctrl), true) }.is_err() {
        eprintln!("Ctrl+C will end the program without cleaning up");
    }
    println!("Expected in Vestigium: {}.", scenario.expected());
    match scenario.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// True once Ctrl+C was pressed.
pub fn stopped() -> bool {
    STOP.load(Ordering::Relaxed)
}

/// Sleeps in short steps so Ctrl+C is noticed.
pub fn sleep_until_stopped(span: Duration) {
    let deadline = std::time::Instant::now() + span;
    while std::time::Instant::now() < deadline && !stopped() {
        std::thread::sleep(Duration::from_millis(100));
    }
}

unsafe extern "system" fn on_ctrl(_kind: u32) -> BOOL {
    STOP.store(true, Ordering::Relaxed);
    true.into()
}

fn usage() -> String {
    "usage: vestigium-scenarios <scenario> [options]
  cpu-burn         [--seconds 75]
  memory-pressure  [--hold-secs 150] [--avail-pct 7]
  disk-saturate    --file <path> [--seconds 45] [--threads 16] [--create-gb N]
  io-burst         --file <path on the system disk> [--seconds 20] [--create-gb N]
  hang-window      [--seconds 12]
  starve           [--seconds 3]   (elevated terminal; never more than 5 s)
  frame-hitch      [--seconds 60] [--every 3] [--hitch-ms 120]"
        .to_string()
}

fn parse(args: &[String]) -> Result<Scenario, String> {
    let (name, rest) = args.split_first().ok_or_else(usage)?;
    let mut options = Options::parse(rest)?;
    let scenario = match name.as_str() {
        "cpu-burn" => Scenario::CpuBurn {
            seconds: options.number("--seconds", 75)?.clamp(1, 600),
        },
        "memory-pressure" => Scenario::MemoryPressure {
            hold_secs: options.number("--hold-secs", 150)?.clamp(1, 600),
            avail_pct: options.float("--avail-pct", 7.0)?.clamp(1.0, 50.0),
        },
        "disk-saturate" => Scenario::DiskSaturate {
            file: options.file()?,
            seconds: options.number("--seconds", 45)?.clamp(1, 600),
            threads: options.number("--threads", 16)?.clamp(1, 64) as usize,
            create_gb: options.optional("--create-gb")?,
        },
        "io-burst" => Scenario::IoBurst {
            file: options.file()?,
            seconds: options.number("--seconds", 20)?.clamp(1, 300),
            create_gb: options.optional("--create-gb")?,
        },
        // Windows calls a window hung after 5 s without messages, so less proves nothing.
        "hang-window" => Scenario::HangWindow {
            seconds: options.number("--seconds", 12)?.clamp(6, 120),
        },
        "starve" => Scenario::Starve {
            seconds: options.number("--seconds", 3)?.clamp(1, 5),
        },
        "frame-hitch" => Scenario::FrameHitch {
            seconds: options.number("--seconds", 60)?.clamp(5, 600),
            every_secs: options.number("--every", 3)?.clamp(1, 60),
            hitch_ms: options.number("--hitch-ms", 120)?.clamp(20, 2000),
        },
        other => return Err(format!("unknown scenario {other}\n{}", usage())),
    };
    options.finish()?;
    Ok(scenario)
}

/// `--name value` pairs, each taken at most once.
struct Options(Vec<(String, String)>);

impl Options {
    fn parse(args: &[String]) -> Result<Options, String> {
        let mut pairs = Vec::new();
        let mut args = args.iter();
        while let Some(flag) = args.next() {
            if !flag.starts_with("--") {
                return Err(format!("unexpected argument {flag}\n{}", usage()));
            }
            let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
            pairs.push((flag.clone(), value.clone()));
        }
        Ok(Options(pairs))
    }

    fn take(&mut self, flag: &str) -> Option<String> {
        let at = self.0.iter().position(|(name, _)| name == flag)?;
        Some(self.0.remove(at).1)
    }

    fn optional(&mut self, flag: &str) -> Result<Option<u64>, String> {
        self.take(flag)
            .map(|value| {
                value
                    .parse()
                    .map_err(|_| format!("{flag} needs a whole number, not {value}"))
            })
            .transpose()
    }

    fn number(&mut self, flag: &str, default: u64) -> Result<u64, String> {
        Ok(self.optional(flag)?.unwrap_or(default))
    }

    fn float(&mut self, flag: &str, default: f64) -> Result<f64, String> {
        match self.take(flag) {
            Some(value) => value
                .parse()
                .map_err(|_| format!("{flag} needs a number, not {value}")),
            None => Ok(default),
        }
    }

    fn file(&mut self) -> Result<PathBuf, String> {
        self.take("--file")
            .map(PathBuf::from)
            .ok_or_else(|| "--file is required".to_string())
    }

    fn finish(self) -> Result<(), String> {
        match self.0.first() {
            Some((flag, _)) => Err(format!("unknown option {flag}\n{}", usage())),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn scenario_args_parse() {
        assert_eq!(
            parse(&args(&["cpu-burn"])),
            Ok(Scenario::CpuBurn { seconds: 75 })
        );
        assert_eq!(
            parse(&args(&[
                "memory-pressure",
                "--hold-secs",
                "30",
                "--avail-pct",
                "9"
            ])),
            Ok(Scenario::MemoryPressure {
                hold_secs: 30,
                avail_pct: 9.0
            })
        );
        assert_eq!(
            parse(&args(&[
                "disk-saturate",
                "--file",
                r"D:\scratch\load.bin",
                "--threads",
                "4",
                "--create-gb",
                "2"
            ])),
            Ok(Scenario::DiskSaturate {
                file: PathBuf::from(r"D:\scratch\load.bin"),
                seconds: 45,
                threads: 4,
                create_gb: Some(2),
            })
        );
        assert_eq!(
            parse(&args(&["hang-window", "--seconds", "20"])),
            Ok(Scenario::HangWindow { seconds: 20 })
        );
        assert!(
            parse(&args(&["disk-saturate"])).is_err(),
            "--file is required"
        );
        assert!(parse(&args(&["io-burst", "--seconds", "soon"])).is_err());
        assert!(parse(&args(&["cpu-burn", "--sconds", "5"])).is_err());
        assert!(parse(&args(&["melt-cpu"])).is_err());
        assert!(parse(&args(&[])).is_err());
    }

    #[test]
    fn starve_caps_its_duration() {
        assert_eq!(
            parse(&args(&["starve", "--seconds", "30"])),
            Ok(Scenario::Starve { seconds: 5 })
        );
        assert_eq!(
            parse(&args(&["starve"])),
            Ok(Scenario::Starve { seconds: 3 })
        );
    }

    #[test]
    fn every_scenario_has_a_ceiling() {
        assert_eq!(
            parse(&args(&["cpu-burn", "--seconds", "100000"])),
            Ok(Scenario::CpuBurn { seconds: 600 })
        );
        assert_eq!(
            parse(&args(&["hang-window", "--seconds", "100000"])),
            Ok(Scenario::HangWindow { seconds: 120 })
        );
        assert_eq!(
            parse(&args(&["hang-window", "--seconds", "2"])),
            Ok(Scenario::HangWindow { seconds: 6 })
        );
    }
}
