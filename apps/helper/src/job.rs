use std::path::PathBuf;

/// Exit code for a job the helper does not run.
pub const REFUSED: u8 = 2;
/// Exit code for a job that ran but could not write its result.
pub const WRITE_FAILED: u8 = 3;
pub const MINUTES: std::ops::RangeInclusive<u64> = 1..=vestigium_core::CAPTURE_MINUTES_MAX;
/// Physical drive numbers a disk scan accepts.
pub const DISKS: std::ops::RangeInclusive<u32> = 0..=127;

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
    Tweak { plan: PathBuf, sha256: String },
    /// Reads physical drive `disk` from `from_mib` to its end, never writing, into `out`.
    DiskScan {
        disk: u32,
        from_mib: u64,
        out: PathBuf,
    },
    /// Checks the file system of volume `letter` online, without repairing, into `out`.
    VolumeCheck { letter: char, out: PathBuf },
}

/// The drive letter of `C:`, and nothing else.
fn volume_letter(text: &str) -> Result<char, String> {
    match text.as_bytes() {
        [letter, b':'] if letter.is_ascii_alphabetic() => {
            Ok(char::from(letter.to_ascii_uppercase()))
        }
        _ => Err(format!("{text} is not a drive letter such as C:")),
    }
}

/// `--flag value` pairs, each flag at most once and from `known` only.
fn pairs<'a>(rest: &'a [String], known: &[&str]) -> Result<Vec<(&'a str, &'a str)>, String> {
    let mut found: Vec<(&str, &str)> = Vec::new();
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
        if !known.contains(&flag.as_str()) || found.iter().any(|(f, _)| f == flag) {
            return Err(format!("unexpected {flag}"));
        }
        found.push((flag, value));
    }
    Ok(found)
}

fn value<'a>(found: &[(&str, &'a str)], flag: &str) -> Option<&'a str> {
    found.iter().find(|(f, _)| *f == flag).map(|(_, v)| *v)
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
        let found = pairs(rest, &["--plan", "--sha256"])?;
        let sha256 = value(&found, "--sha256").ok_or("--sha256 is required")?;
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("--sha256 takes 64 hexadecimal digits".to_string());
        }
        return Ok(Job::Tweak {
            plan: PathBuf::from(value(&found, "--plan").ok_or("--plan is required")?),
            sha256: sha256.to_ascii_lowercase(),
        });
    }
    if name == "disk-scan" {
        let found = pairs(rest, &["--disk", "--out", "--from"])?;
        let disk = value(&found, "--disk")
            .ok_or("--disk is required")?
            .parse::<u32>()
            .map_err(|e| e.to_string())?;
        if !DISKS.contains(&disk) {
            return Err(format!("--disk must be 0 to 127, not {disk}"));
        }
        let from_mib = value(&found, "--from")
            .map(|v| v.parse::<u64>().map_err(|e| e.to_string()))
            .transpose()?
            .unwrap_or(0);
        return Ok(Job::DiskScan {
            disk,
            from_mib,
            out: PathBuf::from(value(&found, "--out").ok_or("--out is required")?),
        });
    }
    if name == "volume-check" {
        let found = pairs(rest, &["--volume", "--out"])?;
        return Ok(Job::VolumeCheck {
            letter: volume_letter(value(&found, "--volume").ok_or("--volume is required")?)?,
            out: PathBuf::from(value(&found, "--out").ok_or("--out is required")?),
        });
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
        return Err(format!(
            "--minutes must be 1 to {}, not {minutes}",
            vestigium_core::CAPTURE_MINUTES_MAX
        ));
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
        assert!(parse(&args(&["tweak", "--plan", "a"])).is_err());
        assert!(parse(&args(&[])).is_err());
    }

    #[test]
    fn tweak_takes_a_plan_and_its_hash() {
        let hash = "AB".repeat(32);
        assert_eq!(
            parse(&args(&["tweak", "--plan", "a", "--sha256", &hash])),
            Ok(Job::Tweak {
                plan: PathBuf::from("a"),
                sha256: "ab".repeat(32),
            })
        );
        for bad in ["", "abc", &"zz".repeat(32), &"ab".repeat(33)] {
            assert!(parse(&args(&["tweak", "--plan", "a", "--sha256", bad])).is_err());
        }
    }

    #[test]
    fn minutes_out_of_range_is_refused() {
        for m in ["0", "1441", "-1", "five"] {
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
    fn disk_scan_takes_a_disk_an_output_and_a_start() {
        assert_eq!(
            parse(&args(&[
                "disk-scan",
                "--disk",
                "1",
                "--out",
                r"C:\a.surface"
            ])),
            Ok(Job::DiskScan {
                disk: 1,
                from_mib: 0,
                out: PathBuf::from(r"C:\a.surface")
            })
        );
        assert_eq!(
            parse(&args(&[
                "disk-scan",
                "--from",
                "4096",
                "--out",
                "o",
                "--disk",
                "0"
            ])),
            Ok(Job::DiskScan {
                disk: 0,
                from_mib: 4096,
                out: PathBuf::from("o")
            })
        );
        for bad in [
            &["disk-scan", "--disk", "1"][..],
            &["disk-scan", "--out", "o"],
            &["disk-scan", "--disk", "128", "--out", "o"],
            &["disk-scan", "--disk", "-1", "--out", "o"],
            &["disk-scan", "--disk", r"\\.\C:", "--out", "o"],
            &["disk-scan", "--disk", "1", "--out", "o", "--disk", "2"],
            &["disk-scan", "--disk", "1", "--out", "o", "--write", "yes"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn volume_check_takes_only_a_drive_letter() {
        assert_eq!(
            parse(&args(&["volume-check", "--volume", "d:", "--out", "o"])),
            Ok(Job::VolumeCheck {
                letter: 'D',
                out: PathBuf::from("o")
            })
        );
        for bad in [
            &[
                "volume-check",
                "--volume",
                "D:",
                "--out",
                "o",
                "--fix",
                "yes",
            ][..],
            &["volume-check", "--volume", "D:\\", "--out", "o"],
            &["volume-check", "--volume", "D: /f", "--out", "o"],
            &["volume-check", "--volume", "1:", "--out", "o"],
            &["volume-check", "--out", "o"],
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
