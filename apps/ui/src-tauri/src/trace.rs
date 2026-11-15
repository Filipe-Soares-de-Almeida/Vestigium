use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Serialize;

/// The longest the helper may take to copy the boot trace.
pub const TRACE_CEILING: Duration = Duration::from_secs(2 * 60);
/// How many copies of the boot trace stay in the folder.
pub const TRACES_KEPT: usize = 3;
/// The folder of the data folder that holds the copies.
pub const TRACE_FOLDER: &str = "boot-traces";

/// What the window gets back when the helper that copies the boot trace exits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BootTraceCopied {
    /// The copy, when the helper reported success and the file is there.
    pub path: Option<String>,
    /// 0 copied, 6 Windows has no trace, anything else failed.
    pub exit_code: Option<u32>,
}

/// `<data>\boot-traces\<start ms>.etl`, beside the watcher's database.
pub fn boot_trace_file(db: &Path, started_ms: i64) -> Option<PathBuf> {
    Some(
        db.parent()?
            .join(TRACE_FOLDER)
            .join(format!("{started_ms}.etl")),
    )
}

/// The helper's arguments for copying the boot trace into `out`.
pub fn boot_trace_args(out: &Path) -> String {
    format!("boot-trace --out \"{}\"", out.display())
}

/// The number in the name of a `<digits>.etl` file; any other name has none.
fn trace_number(name: &str) -> Option<u128> {
    let (stem, extension) = name.split_once('.')?;
    if !extension.eq_ignore_ascii_case("etl") || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    stem.parse().ok()
}

/// Deletes all but the `keep` newest `<digits>.etl` files of `folder`, newest being the largest
/// number in the name, and returns how many it deleted. Anything else in the folder stays.
pub fn prune_boot_traces(folder: &Path, keep: usize) -> usize {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return 0;
    };
    let mut copies: Vec<(u128, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let number = trace_number(entry.file_name().to_str()?)?;
            Some((number, entry.path()))
        })
        .collect();
    copies.sort_by(|a, b| b.cmp(a));
    copies
        .into_iter()
        .skip(keep)
        .filter(|(_, path)| std::fs::remove_file(path).is_ok())
        .count()
}

/// How long a partial copy may sit before it counts as left by a helper that never finished.
pub const PARTIAL_STALE: Duration = Duration::from_secs(60 * 60);

/// Deletes the `<digits>.etl.tmp` files of `folder` last written more than `stale` before `now`.
pub fn drop_stale_partials(folder: &Path, stale: Duration, now: SystemTime) -> usize {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let partial = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_suffix(".tmp"))
                .is_some_and(|name| trace_number(name).is_some());
            let old = entry
                .metadata()
                .ok()
                .filter(|meta| meta.is_file())
                .and_then(|meta| meta.modified().ok())
                .and_then(|written| now.duration_since(written).ok())
                .is_some_and(|age| age > stale);
            partial && old
        })
        .filter(|entry| std::fs::remove_file(entry.path()).is_ok())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(folder: &Path) -> Vec<String> {
        let mut all: Vec<String> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        all.sort();
        all
    }

    #[test]
    fn the_trace_file_and_args_sit_beside_the_database() {
        let db = Path::new(r"C:\Users\some one\AppData\Local\Vestigium\data\vestigium.db");
        let out = boot_trace_file(db, 7).unwrap();
        assert_eq!(
            out,
            PathBuf::from(r"C:\Users\some one\AppData\Local\Vestigium\data\boot-traces\7.etl")
        );
        assert_eq!(
            boot_trace_args(&out),
            r#"boot-trace --out "C:\Users\some one\AppData\Local\Vestigium\data\boot-traces\7.etl""#
        );
    }

    #[test]
    fn the_three_newest_copies_stay_and_the_older_ones_go() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["9.etl", "10.etl", "100.etl", "5.etl", "99.etl"] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        assert_eq!(prune_boot_traces(dir.path(), TRACES_KEPT), 2);
        assert_eq!(names(dir.path()), ["10.etl", "100.etl", "99.etl"]);
    }

    #[test]
    fn nothing_that_is_not_a_numbered_copy_is_touched() {
        let dir = tempfile::tempdir().unwrap();
        let others = [
            "notes.txt",
            "abc.etl",
            "1.etl.tmp",
            "1.ETL.bak",
            "1.etx",
            "etl",
            ".etl",
            "1a.etl",
            "99999999999999999999999999999999999999999999999.etl",
        ];
        for name in others {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        std::fs::create_dir(dir.path().join("5.etl.d")).unwrap();
        std::fs::create_dir(dir.path().join("6.etl")).unwrap();
        for name in ["1.etl", "2.etl", "3.etl", "4.etl"] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        assert_eq!(prune_boot_traces(dir.path(), TRACES_KEPT), 1);
        let mut left = names(dir.path());
        left.retain(|name| name != "2.etl" && name != "3.etl" && name != "4.etl");
        let mut expected: Vec<String> = others.iter().map(|name| name.to_string()).collect();
        expected.extend(["5.etl.d".to_string(), "6.etl".to_string()]);
        expected.sort();
        assert_eq!(left, expected);
    }

    #[test]
    fn a_folder_with_few_copies_or_none_loses_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(prune_boot_traces(dir.path(), TRACES_KEPT), 0);
        assert_eq!(
            prune_boot_traces(&dir.path().join("missing"), TRACES_KEPT),
            0
        );
        std::fs::write(dir.path().join("1.etl"), b"x").unwrap();
        std::fs::write(dir.path().join("2.etl"), b"x").unwrap();
        assert_eq!(prune_boot_traces(dir.path(), TRACES_KEPT), 0);
        assert_eq!(names(dir.path()), ["1.etl", "2.etl"]);
    }

    #[test]
    fn only_old_partial_copies_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["1.etl.tmp", "2.etl", "notes.tmp", "x.etl.tmp"] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        let now = SystemTime::now();
        assert_eq!(drop_stale_partials(dir.path(), PARTIAL_STALE, now), 0);
        let later = now + PARTIAL_STALE + Duration::from_secs(60);
        assert_eq!(drop_stale_partials(dir.path(), PARTIAL_STALE, later), 1);
        assert_eq!(names(dir.path()), ["2.etl", "notes.tmp", "x.etl.tmp"]);
    }

    #[test]
    fn leading_zeros_count_by_their_number() {
        assert_eq!(trace_number("007.etl"), Some(7));
        assert_eq!(trace_number("7.ETL"), Some(7));
        assert_eq!(trace_number("7.etl.tmp"), None);
        assert_eq!(trace_number(".etl"), None);
    }
}
