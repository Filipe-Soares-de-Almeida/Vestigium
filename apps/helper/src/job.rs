use std::path::PathBuf;

/// Exit code for a job the helper does not run.
pub const REFUSED: u8 = 2;
/// Exit code for a job that ran but could not write its result.
pub const WRITE_FAILED: u8 = 3;
pub const MINUTES: std::ops::RangeInclusive<u64> = 1..=60;

/// The closed list of jobs.
#[derive(Debug, PartialEq, Eq)]
pub enum Job {
    /// Traces DPC/ISR time per driver and frame times for `minutes`, into `out`; `etl` keeps the raw trace.
    DeepCapture {
        minutes: u64,
        out: PathBuf,
        etl: Option<PathBuf>,
    },
    /// Reads the boot diagnostics, which need elevation, into `out`.
    HealthScan { out: PathBuf },
    /// Reads what every crash dump Windows kept says, into `out`.
    DumpScan { out: PathBuf },
    /// Deletes the dumps a list file names, each checked again.
    DeleteDumps { list: PathBuf },
    /// Applies or undoes the tweaks a plan file names, each checked against the catalog.
    Tweak { plan: PathBuf },
}

/// The job these arguments name, or why they name none.
pub fn parse(args: &[String]) -> Result<Job, String> {
    let (name, rest) = args.split_first().ok_or("no job")?;
    if name == "health-scan" || name == "dump-scan" {
        let [flag, out] = rest else {
            return Err(format!("{name} takes --out <file> and nothing else"));
        };
        if flag != "--out" {
            return Err(format!("{name} takes --out <file> and nothing else"));
        }
        let out = PathBuf::from(out);
        return Ok(if name == "health-scan" {
            Job::HealthScan { out }
        } else {
            Job::DumpScan { out }
        });
    }
    if name == "delete-dumps" {
        return match rest {
            [flag, list] if flag == "--list" => Ok(Job::DeleteDumps {
                list: PathBuf::from(list),
            }),
            _ => Err("delete-dumps takes --list <file> and nothing else".to_string()),
        };
    }
    if name == "tweak" {
        return match rest {
            [flag, plan] if flag == "--plan" => Ok(Job::Tweak {
                plan: PathBuf::from(plan),
            }),
            _ => Err("tweak takes --plan <file> and nothing else".to_string()),
        };
    }
    if name != "deep-capture" {
        return Err(format!("unknown job {name}"));
    }
    let (mut minutes, mut out, mut etl) = (None, None, None);
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--minutes" if minutes.is_none() => {
                minutes = Some(value.parse::<u64>().map_err(|e| e.to_string())?)
            }
            "--out" if out.is_none() => out = Some(PathBuf::from(value)),
            "--etl" if etl.is_none() => etl = Some(PathBuf::from(value)),
            other => return Err(format!("unexpected {other}")),
        }
    }
    let minutes = minutes.ok_or("--minutes is required")?;
    if !MINUTES.contains(&minutes) {
        return Err(format!("--minutes must be 1 to 60, not {minutes}"));
    }
    Ok(Job::DeepCapture {
        minutes,
        out: out.ok_or("--out is required")?,
        etl,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn refuses_unknown_job() {
        assert!(parse(&args(&["wipe-disk", "--out", "x"])).is_err());
        assert!(parse(&args(&["delete-dumps", "--minutes", "5"])).is_err());
        assert!(parse(&args(&["delete-dumps", "--list", "a", "--list", "b"])).is_err());
        assert!(parse(&args(&["tweak", "--list", "a"])).is_err());
        assert!(parse(&args(&["tweak", "--plan", "a", "--plan", "b"])).is_err());
        assert!(parse(&args(&[])).is_err());
    }

    #[test]
    fn minutes_out_of_range_is_refused() {
        for m in ["0", "61", "-1", "five"] {
            assert!(parse(&args(&["deep-capture", "--minutes", m, "--out", "x"])).is_err());
        }
    }

    #[test]
    fn repeated_or_unknown_flags_are_refused() {
        assert!(
            parse(&args(&[
                "deep-capture",
                "--minutes",
                "5",
                "--minutes",
                "6",
                "--out",
                "x"
            ]))
            .is_err()
        );
        assert!(
            parse(&args(&[
                "deep-capture",
                "--minutes",
                "5",
                "--run",
                "cmd.exe"
            ]))
            .is_err()
        );
    }

    #[test]
    fn health_scan_takes_only_its_output() {
        assert_eq!(
            parse(&args(&["health-scan", "--out", r"C:\a.scan"])),
            Ok(Job::HealthScan {
                out: PathBuf::from(r"C:\a.scan")
            })
        );
        for bad in [
            &["health-scan"][..],
            &["health-scan", "--out"],
            &["health-scan", "--minutes", "5"],
            &["health-scan", "--out", "a", "--out", "b"],
            &["health-scan", "--out", "a", "--run", "cmd.exe"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn dump_scan_takes_only_its_output() {
        assert_eq!(
            parse(&args(&["dump-scan", "--out", r"C:\a.dumps"])),
            Ok(Job::DumpScan {
                out: PathBuf::from(r"C:\a.dumps")
            })
        );
        for bad in [
            &["dump-scan"][..],
            &["dump-scan", "--out", "a", "--path", r"C:\Windows"],
            &["dump-scan", "--path", r"C:\Windows\MEMORY.DMP"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn deep_capture_parses() {
        assert_eq!(
            parse(&args(&[
                "deep-capture",
                "--minutes",
                "5",
                "--out",
                r"C:\a.capture"
            ])),
            Ok(Job::DeepCapture {
                minutes: 5,
                out: PathBuf::from(r"C:\a.capture"),
                etl: None
            })
        );
    }
}
