use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vestigium_collect::{
    BenchTarget, LockedRegion, Words, benchmark, memory_diagnostic_events, memory_test,
    run_patterns,
};
use vestigium_core::hwtest::{
    BENCH_FILE_BYTES, BenchMode, Bus, ERRORS_KEPT, Outcome, PATTERNS, Pattern, QUICK_CHUNK,
    TestData,
};
use vestigium_core::{EventMeaning, Media};

/// Memory with one word whose bit 5 always reads back inverted.
struct Faulty {
    words: Vec<u64>,
    bad: usize,
}

impl Words for Faulty {
    fn len(&self) -> usize {
        self.words.len()
    }

    fn put(&mut self, index: usize, value: u64) {
        self.words[index] = value;
    }

    fn get(&self, index: usize) -> u64 {
        let value = self.words[index];
        if index == self.bad {
            value ^ 1 << 5
        } else {
            value
        }
    }
}

/// One round of every pattern, with the errors each pattern added.
fn one_round<W: Words + ?Sized>(mem: &mut W) -> (vestigium_collect::PatternRun, Vec<u64>) {
    let stop = AtomicBool::new(false);
    let mut found = Vec::new();
    let mut before = 0;
    let run = run_patterns(
        mem,
        Instant::now() + Duration::from_secs(600),
        &stop,
        &mut |p| {
            found.push(p.error_count - before);
            before = p.error_count;
            if p.passes as usize == PATTERNS.len() {
                stop.store(true, Ordering::Relaxed);
            }
        },
    );
    (run, found)
}

#[test]
fn every_pattern_finds_a_flipped_bit() {
    let mut mem = Faulty {
        words: vec![0; 1 << 18],
        bad: (1 << 17) + 12_345,
    };
    let (run, found) = one_round(&mut mem);
    assert_eq!(run.passes as usize, PATTERNS.len());
    for (pattern, count) in PATTERNS.iter().zip(&found) {
        assert!(*count > 0, "{pattern:?} missed the bad word");
    }
    assert!(run.errors.len() <= ERRORS_KEPT);
    // The compare pattern changes words in place, so one bad bit spreads through the arithmetic.
    assert!(
        run.errors
            .iter()
            .filter(|e| e.pattern != Pattern::Compare)
            .all(|e| e.flipped_bits() == 1)
    );
    let bad = mem.bad as u64 * 8;
    assert!(run.errors.iter().all(|e| e.offset == bad));
}

#[test]
fn good_memory_has_no_errors() {
    let mut words = vec![0u64; 1 << 18];
    let (run, _) = one_round(words.as_mut_slice());
    assert_eq!(run.error_count, 0);
    assert!(run.stopped);
}

#[test]
fn a_past_deadline_stops_before_the_first_pattern() {
    let mut words = vec![0u64; 1 << 18];
    let stop = AtomicBool::new(false);
    let run = run_patterns(words.as_mut_slice(), Instant::now(), &stop, &mut |_| {});
    assert_eq!((run.rounds, run.passes, run.stopped), (0, 0, false));
}

#[test]
fn a_locked_piece_is_released() {
    let stop = AtomicBool::new(false);
    let mut region = LockedRegion::lock(QUICK_CHUNK, &stop).expect("one piece locks");
    assert_eq!(region.bytes(), QUICK_CHUNK);
    assert_eq!(region.words().len() as u64 * 8, QUICK_CHUNK);
    drop(region);
}

#[test]
fn memory_test_reports_what_it_covered() {
    let stop = AtomicBool::new(false);
    let run = memory_test(QUICK_CHUNK, Duration::from_millis(300), &stop, &mut |_| {});
    assert_eq!(run.outcome, Outcome::Clean);
    let TestData::MemoryQuick(quick) = run.data else {
        panic!("a memory test");
    };
    assert_eq!(quick.tested_bytes, QUICK_CHUNK);
    assert!(quick.total_bytes >= QUICK_CHUNK);
    assert!(run.ended_ms >= run.started_ms);
}

#[test]
fn benchmark_measures_every_mode_and_leaves_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = BenchTarget {
        dir: dir.path().to_path_buf(),
        volume: "C:\\".into(),
        disk: None,
        bus: Bus::Other,
        media: Media::Unknown,
    };
    let stop = AtomicBool::new(false);
    let mut seen = Vec::new();
    let run = benchmark(&target, 16 << 20, 0.2, &stop, &mut |mode| seen.push(mode));
    assert_eq!(run.outcome, Outcome::Clean, "{:?}", run.data);
    let TestData::DiskBenchmark(bench) = run.data else {
        panic!("a benchmark");
    };
    assert_eq!(seen, BenchMode::ALL);
    assert_eq!(bench.results.len(), BenchMode::ALL.len());
    assert!(bench.written_bytes >= 16 << 20);
    assert!(bench.results.iter().all(|r| r.mb_per_s > 0.0));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn benchmark_refuses_a_file_larger_than_the_free_space() {
    let dir = tempfile::tempdir().unwrap();
    let target = BenchTarget {
        dir: dir.path().to_path_buf(),
        volume: "C:\\".into(),
        disk: None,
        bus: Bus::Other,
        media: Media::Unknown,
    };
    let stop = AtomicBool::new(false);
    let mut seen = Vec::new();
    let run = benchmark(&target, 1 << 60, 0.2, &stop, &mut |mode| seen.push(mode));
    assert_eq!(run.outcome, Outcome::Failed);
    let TestData::DiskBenchmark(bench) = run.data else {
        panic!("a benchmark");
    };
    assert!(bench.failure.is_some_and(|f| f.contains("free space")));
    assert_eq!((bench.written_bytes, seen.len()), (0, 0));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn memory_diagnostic_events_read_without_elevation() {
    let events = memory_diagnostic_events().expect("the System log answers");
    assert!(
        events
            .iter()
            .all(|e| e.meaning == EventMeaning::MemoryDiagnostic)
    );
}

#[test]
fn disk_and_volume_facts_read_without_elevation() {
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let root = format!("{system}\\");
    assert_eq!(
        vestigium_collect::file_system(&root).as_deref(),
        Some("NTFS")
    );
    let disk = vestigium_collect::volume_drive(&system).expect("the system volume has a disk");
    assert!(vestigium_collect::disk_size(disk).is_some_and(|b| b > 1 << 30));
}

/// A small NTFS volume on a virtual disk, detached and deleted when dropped.
struct SmallVolume {
    vhd: PathBuf,
    letter: char,
}

impl SmallVolume {
    fn diskpart(script: &str, folder: &Path) -> bool {
        let file = folder.join("diskpart.txt");
        std::fs::write(&file, script).unwrap();
        Command::new("diskpart.exe")
            .arg("/s")
            .arg(&file)
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn new(folder: &Path, megabytes: u32, letter: char) -> SmallVolume {
        let vhd = folder.join("small.vhd");
        let script = format!(
            "create vdisk file=\"{v}\" maximum={megabytes} type=fixed
select vdisk file=\"{v}\"
             attach vdisk
create partition primary
format fs=ntfs quick label=VBENCH
             assign letter={letter}
",
            v = vhd.display()
        );
        let volume = SmallVolume { vhd, letter };
        assert!(Self::diskpart(&script, folder), "diskpart made the volume");
        volume
    }

    fn root(&self) -> PathBuf {
        PathBuf::from(format!("{}:\\", self.letter))
    }
}

impl Drop for SmallVolume {
    fn drop(&mut self) {
        let folder = self.vhd.parent().unwrap().to_path_buf();
        let script = format!(
            "select vdisk file=\"{}\"
detach vdisk
",
            self.vhd.display()
        );
        let _ = Self::diskpart(&script, &folder);
        let _ = std::fs::remove_file(&self.vhd);
    }
}

/// Elevated: makes a 900 MiB volume, so it is ignored by default. Run it as administrator with
/// `cargo test -p vestigium-collect --test hwtest -- --ignored benchmark_refuses_a_volume_without_room`.
#[test]
#[ignore = "needs elevation and creates a 900 MiB virtual disk"]
fn benchmark_refuses_a_volume_without_room() {
    let folder = tempfile::tempdir().unwrap();
    let letter = ('H'..='Z')
        .rev()
        .find(|c| !Path::new(&format!("{c}:\\")).exists())
        .expect("a free drive letter");
    let volume = SmallVolume::new(folder.path(), 900, letter);
    let target = BenchTarget {
        dir: volume.root(),
        volume: volume.root().display().to_string(),
        disk: None,
        bus: Bus::Other,
        media: Media::Unknown,
    };
    let stop = AtomicBool::new(false);
    let run = benchmark(&target, BENCH_FILE_BYTES, 0.2, &stop, &mut |_| {});
    assert_eq!(run.outcome, Outcome::Failed);
    let TestData::DiskBenchmark(bench) = run.data else {
        panic!("a benchmark");
    };
    assert!(bench.failure.is_some_and(|f| f.contains("free space")));
    assert_eq!(bench.written_bytes, 0);
    let left = std::fs::read_dir(volume.root())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("vestigium-bench")
        })
        .count();
    assert_eq!(left, 0);
}
