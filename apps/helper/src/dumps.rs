use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use vestigium_collect::{
    DumpFile, app_dump_dir, crash_control, dump_file, list_dumps_one_level, wall_ms,
};
use vestigium_core::{
    Analysis, DUMP_SCAN_VERSION, DumpKind, DumpRecord, DumpScan, Fault, fault_address,
    kernel_dump_kind,
};
use windows::Win32::System::Diagnostics::Debug::Extensions::{
    DEBUG_EVENT_EXCEPTION, DEBUG_LAST_EVENT_INFO_EXCEPTION, DEBUG_MODNAME_IMAGE,
    DEBUG_MODULE_PARAMETERS, DEBUG_STACK_FRAME, DEBUG_WAIT_DEFAULT, DebugCreate, IDebugClient4,
    IDebugControl4, IDebugDataSpaces, IDebugRegisters, IDebugSymbols3,
};
use windows::core::{HSTRING, Interface, s};

use crate::{job, output};

/// Most dumps one read looks at, newest first.
const MOST_DUMPS: usize = 200;
/// Bytes of the faulting thread's stack scanned for module addresses.
const STACK_BYTES: usize = 0x3000;
const FRAMES: usize = 64;
/// Size of the header of a kernel dump.
const KERNEL_HEADER: usize = 0x2000;
const FILETIME_UNIX_DIFF_MS: i64 = 11_644_473_600_000;
/// Longest the debugger engine may take to load one dump, in milliseconds.
const WAIT_FOR_DUMP_MS: u32 = 60_000;
/// Longest the whole scan may spend in the engine; later dumps keep their size and date only.
const SCAN_CEILING: Duration = Duration::from_secs(600);

/// What the first bytes of a kernel dump say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelHeader {
    pub code: u32,
    pub params: [u64; 4],
    pub dump_type: u32,
    /// When the PC stopped, Unix epoch ms.
    pub crash_ms: i64,
}

/// Reads every dump in the folders Windows keeps them in and writes what each says to `out`;
/// a dump the engine cannot read is kept with its size and date only.
pub fn run(out: &Path) -> ExitCode {
    let Ok(mut file) = output::create(out, "dumps", "dumps") else {
        return ExitCode::from(job::REFUSED);
    };
    // SAFETY: the helper is single-threaded here; the engine must never look for symbols online.
    unsafe {
        std::env::remove_var("_NT_SYMBOL_PATH");
        std::env::remove_var("_NT_ALT_SYMBOL_PATH");
    }
    let mut found = dump_files();
    found.sort_by_key(|(f, _)| std::cmp::Reverse(f.file_ms));
    found.truncate(MOST_DUMPS);
    let started = Instant::now();
    let scan = DumpScan {
        ts_ms: wall_ms(),
        dumps: found
            .into_iter()
            .map(|(f, app)| read_dump(f, app, started.elapsed() < SCAN_CEILING))
            .collect(),
    };
    let mut bytes = vec![DUMP_SCAN_VERSION];
    match postcard::to_allocvec(&scan) {
        Ok(body) => bytes.extend(body),
        Err(_) => return ExitCode::from(job::WRITE_FAILED),
    }
    if file.write_all(&bytes).is_err() || file.commit().is_err() {
        return ExitCode::from(job::WRITE_FAILED);
    }
    ExitCode::SUCCESS
}

/// The dump files Windows keeps, each with whether it is an app's.
fn dump_files() -> Vec<(DumpFile, bool)> {
    let mut files = Vec::new();
    if let Some(crash) = crash_control() {
        files.extend(dump_file(&crash.dump_file).map(|f| (f, false)));
        files.extend(listed(&crash.minidump_dir, false));
    }
    if let Some(root) = std::env::var_os("SystemRoot") {
        files.extend(listed(
            &PathBuf::from(root).join("LiveKernelReports"),
            false,
        ));
    }
    if let Some(dir) = app_dump_dir() {
        files.extend(listed(&dir, true));
    }
    files
}

/// The `.dmp` files of `dir` and of the folders right under it.
fn listed(dir: &Path, app: bool) -> Vec<(DumpFile, bool)> {
    list_dumps_one_level(dir)
        .map(|listing| listing.files)
        .unwrap_or_default()
        .into_iter()
        .map(|f| (f, app))
        .collect()
}

fn read_dump(file: DumpFile, app: bool, analyse: bool) -> DumpRecord {
    let path = file.path.to_string_lossy().into_owned();
    let mut head = vec![0u8; KERNEL_HEADER];
    let read = File::open(&file.path)
        .and_then(|mut f| f.read(&mut head))
        .unwrap_or(0);
    head.truncate(read);
    let kind = if app {
        DumpKind::App
    } else {
        kernel_dump_kind(&path)
    };
    let crash_ms = kernel_header(&head)
        .map(|h| h.crash_ms)
        .or_else(|| app_crash_ms(&head))
        .unwrap_or(file.file_ms);
    let process = app.then(|| process_name(&file.path)).flatten();
    DumpRecord {
        path,
        file_ms: file.file_ms,
        kind,
        crash_ms,
        size_bytes: file.size_bytes,
        analysis: analyse
            .then(|| analyze(&file.path, process, WAIT_FOR_DUMP_MS))
            .flatten(),
    }
}

/// Opens the dump with the debugger engine that ships with Windows, no symbols, no network;
/// gives up after `wait_ms` waiting for the engine to load it.
fn analyze(path: &Path, process: Option<String>, wait_ms: u32) -> Option<Analysis> {
    // SAFETY: plain engine calls on interfaces created here, one engine per dump; every buffer
    // outlives the call that fills it.
    unsafe {
        let client: IDebugClient4 = DebugCreate().ok()?;
        let control: IDebugControl4 = client.cast().ok()?;
        let symbols: IDebugSymbols3 = client.cast().ok()?;
        let data: IDebugDataSpaces = client.cast().ok()?;
        let registers: IDebugRegisters = client.cast().ok()?;
        symbols.SetSymbolPath(s!("")).ok()?;
        symbols.SetImagePath(s!("")).ok()?;
        client
            .OpenDumpFileWide(&HSTRING::from(path.as_os_str()), 0)
            .ok()?;
        let analysis = (|| {
            control.WaitForEvent(DEBUG_WAIT_DEFAULT, wait_ms).ok()?;
            let fault = fault_of(&control)?;
            let modules = modules_of(&symbols);
            let mut stack = Vec::new();
            let mut frames = [DEBUG_STACK_FRAME::default(); FRAMES];
            let mut filled = 0u32;
            if control
                .GetStackTrace(0, 0, 0, &mut frames, Some(&mut filled))
                .is_ok()
            {
                stack.extend(
                    frames[..filled as usize]
                        .iter()
                        .map(|f| f.InstructionOffset),
                );
            }
            let sp = registers.GetStackOffset().unwrap_or(0);
            let mut raw = vec![0u8; STACK_BYTES];
            let mut read = 0u32;
            let _ = data.ReadVirtual(
                sp,
                raw.as_mut_ptr().cast(),
                STACK_BYTES as u32,
                Some(&mut read),
            );
            stack.extend(
                raw[..read as usize]
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|w| u64::from_le_bytes(*w)),
            );
            let mut stack_modules: Vec<String> = Vec::new();
            for name in stack.iter().filter_map(|a| module_at(&modules, *a)) {
                if !stack_modules.iter().any(|m| m == name) {
                    stack_modules.push(name.to_string());
                }
            }
            Some(Analysis {
                fault,
                fault_module: fault_address(&fault)
                    .and_then(|a| module_at(&modules, a))
                    .map(str::to_string),
                stack_modules,
                process,
                module_count: modules.len() as u32,
            })
        })();
        let _ = client.EndSession(0);
        analysis
    }
}

/// A blue screen's stop code, else the exception an app died of.
///
/// # Safety
/// `control` must hold an opened dump.
unsafe fn fault_of(control: &IDebugControl4) -> Option<Fault> {
    let (mut code, mut a1, mut a2, mut a3, mut a4) = (0u32, 0u64, 0u64, 0u64, 0u64);
    // SAFETY: five plain out-parameters.
    if unsafe { control.ReadBugCheckData(&mut code, &mut a1, &mut a2, &mut a3, &mut a4) }.is_ok() {
        return Some(Fault::BugCheck {
            code,
            params: [a1, a2, a3, a4],
        });
    }
    let (mut kind, mut pid, mut tid) = (0u32, 0u32, 0u32);
    let mut exception = DEBUG_LAST_EVENT_INFO_EXCEPTION::default();
    // SAFETY: the extra-information buffer is a live struct of the size passed.
    unsafe {
        control.GetLastEventInformation(
            &mut kind,
            &mut pid,
            &mut tid,
            Some(&raw mut exception as *mut c_void),
            size_of::<DEBUG_LAST_EVENT_INFO_EXCEPTION>() as u32,
            None,
            None,
            None,
        )
    }
    .ok()?;
    (kind == DEBUG_EVENT_EXCEPTION).then_some(Fault::Exception {
        code: exception.ExceptionRecord.ExceptionCode.0 as u32,
        address: exception.ExceptionRecord.ExceptionAddress,
    })
}

struct Module {
    name: String,
    base: u64,
    size: u64,
}

/// The loaded modules, by image file name.
///
/// # Safety
/// `symbols` must hold an opened dump.
unsafe fn modules_of(symbols: &IDebugSymbols3) -> Vec<Module> {
    let (mut loaded, mut unloaded) = (0u32, 0u32);
    // SAFETY: two plain out-parameters.
    if unsafe { symbols.GetNumberModules(&mut loaded, &mut unloaded) }.is_err() {
        return Vec::new();
    }
    let mut modules = Vec::with_capacity(loaded as usize);
    for index in 0..loaded {
        // SAFETY: an index the engine just counted; the buffers outlive each call.
        let Ok(base) = (unsafe { symbols.GetModuleByIndex(index) }) else {
            continue;
        };
        let mut params = DEBUG_MODULE_PARAMETERS::default();
        // SAFETY: one base in, one parameter block out.
        let _ = unsafe { symbols.GetModuleParameters(1, Some(&base), 0, &mut params) };
        let mut name = [0u8; 520];
        let mut used = 0u32;
        // SAFETY: the buffer and its length are passed together.
        let _ = unsafe {
            symbols.GetModuleNameString(
                DEBUG_MODNAME_IMAGE,
                index,
                0,
                Some(&mut name),
                Some(&mut used),
            )
        };
        let full = String::from_utf8_lossy(&name[..used.saturating_sub(1) as usize]).into_owned();
        modules.push(Module {
            name: file_name(&full),
            base,
            size: u64::from(params.Size),
        });
    }
    modules
}

fn module_at(modules: &[Module], address: u64) -> Option<&str> {
    modules
        .iter()
        .find(|m| address >= m.base && address < m.base + m.size)
        .map(|m| m.name.as_str())
        .filter(|n| !n.is_empty())
}

/// The last part of a Windows path, as an image name.
fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

/// Stop code, parameters, dump type and crash time from a kernel dump's first bytes.
pub fn kernel_header(head: &[u8]) -> Option<KernelHeader> {
    if head.len() < 0x1000 || &head[0..4] != b"PAGE" || &head[4..8] != b"DU64" {
        return None;
    }
    let u32_at = |at: usize| u32::from_le_bytes(head[at..at + 4].try_into().unwrap_or_default());
    let u64_at = |at: usize| u64::from_le_bytes(head[at..at + 8].try_into().unwrap_or_default());
    let filetime = u64_at(0xFA8) as i64;
    Some(KernelHeader {
        code: u32_at(0x38),
        params: [u64_at(0x40), u64_at(0x48), u64_at(0x50), u64_at(0x58)],
        dump_type: u32_at(0xF98),
        crash_ms: filetime / 10_000 - FILETIME_UNIX_DIFF_MS,
    })
}

/// When an app dump was written, from its header, Unix epoch ms.
pub fn app_crash_ms(head: &[u8]) -> Option<i64> {
    if head.len() < 32 || &head[0..4] != b"MDMP" {
        return None;
    }
    let seconds = u32::from_le_bytes(head[20..24].try_into().ok()?);
    (seconds != 0).then(|| i64::from(seconds) * 1000)
}

/// The executable a crash dump file is named after, as `Game.exe.1234.dmp` gives `Game.exe`.
pub fn process_name(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let name = match stem.rsplit_once('.') {
        Some((name, pid)) if !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => stem.as_str(),
    };
    name.to_ascii_lowercase()
        .ends_with(".exe")
        .then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel_head(code: u32, dump_type: u32, filetime: u64) -> Vec<u8> {
        let mut head = vec![0u8; KERNEL_HEADER];
        head[0..4].copy_from_slice(b"PAGE");
        head[4..8].copy_from_slice(b"DU64");
        head[0x38..0x3C].copy_from_slice(&code.to_le_bytes());
        for (i, p) in [0x11u64, 0x22, 0x33, 0x44].iter().enumerate() {
            head[0x40 + i * 8..0x48 + i * 8].copy_from_slice(&p.to_le_bytes());
        }
        head[0xF98..0xF9C].copy_from_slice(&dump_type.to_le_bytes());
        head[0xFA8..0xFB0].copy_from_slice(&filetime.to_le_bytes());
        head
    }

    #[test]
    fn kernel_header_gives_code_type_and_time() {
        // A FILETIME in 2030.
        let filetime = (1_894_609_800_000i64 + FILETIME_UNIX_DIFF_MS) as u64 * 10_000;
        let header = kernel_header(&kernel_head(0x50, 4, filetime)).unwrap();
        assert_eq!(header.code, 0x50);
        assert_eq!(header.params, [0x11, 0x22, 0x33, 0x44]);
        assert_eq!(header.dump_type, 4);
        assert_eq!(header.crash_ms, 1_894_609_800_000);
    }

    #[test]
    fn anything_but_a_kernel_dump_has_no_kernel_header() {
        let mut head = kernel_head(0x50, 4, 0);
        head[4..8].copy_from_slice(b"DUMP");
        assert_eq!(kernel_header(&head), None, "32-bit dumps are not read");
        assert_eq!(kernel_header(b"PAGEDU64"), None, "too short");
    }

    #[test]
    fn app_dump_gives_its_time() {
        let mut head = vec![0u8; 64];
        head[0..4].copy_from_slice(b"MDMP");
        head[20..24].copy_from_slice(&1_894_609_800u32.to_le_bytes());
        assert_eq!(app_crash_ms(&head), Some(1_894_609_800_000));
        head[0..4].copy_from_slice(b"PAGE");
        assert_eq!(app_crash_ms(&head), None);
    }

    #[test]
    fn the_file_name_gives_the_app() {
        assert_eq!(
            process_name(Path::new(r"C:\x\CrashDumps\Game.exe.4120.dmp")),
            Some("Game.exe".into())
        );
        assert_eq!(
            process_name(Path::new(r"C:\x\CrashDumps\Game.exe.dmp")),
            Some("Game.exe".into())
        );
        assert_eq!(process_name(Path::new(r"C:\x\CrashDumps\core.dmp")), None);
        assert_eq!(
            file_name(r"\SystemRoot\system32\ntoskrnl.exe"),
            "ntoskrnl.exe"
        );
    }

    fn dump_folders() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let crash = dir.path().join("CrashDumps");
        std::fs::create_dir_all(crash.join("Real")).unwrap();
        std::fs::create_dir_all(dir.path().join("Other")).unwrap();
        std::fs::write(crash.join("Real").join("1.dmp"), b"x").unwrap();
        std::fs::write(crash.join("Top.exe.1.dmp"), b"x").unwrap();
        std::fs::write(dir.path().join("Other").join("2.dmp"), b"x").unwrap();
        (dir, crash)
    }

    #[test]
    fn listed_skips_a_junction_to_another_dump() {
        let (dir, crash) = dump_folders();
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(crash.join("Linked"))
            .arg(dir.path().join("Other"))
            .output()
            .unwrap();
        assert!(made.status.success(), "could not make a junction");
        let names: Vec<String> = listed(&crash, true)
            .into_iter()
            .map(|(f, _)| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(!names.contains(&"2.dmp".to_string()), "{names:?}");
        assert!(names.contains(&"1.dmp".to_string()), "{names:?}");
    }

    #[test]
    fn listed_keeps_the_dumps_of_plain_folders() {
        let (_dir, crash) = dump_folders();
        let mut names: Vec<String> = listed(&crash, true)
            .into_iter()
            .map(|(f, _)| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["1.dmp", "Top.exe.1.dmp"]);
    }

    fn invented_dump(dir: &Path) -> DumpFile {
        let path = dir.join("Invented.exe.1.dmp");
        let mut bytes = b"MDMP".to_vec();
        bytes.extend((0..4096u32).map(|i| (i % 251) as u8));
        std::fs::write(&path, &bytes).unwrap();
        DumpFile {
            path,
            file_ms: 1_000,
            size_bytes: bytes.len() as u64,
        }
    }

    #[test]
    fn a_dump_past_the_scan_ceiling_keeps_its_size_and_date_only() {
        let dir = tempfile::tempdir().unwrap();
        let record = read_dump(invented_dump(dir.path()), true, false);
        assert!(record.analysis.is_none());
        assert_eq!((record.size_bytes, record.file_ms), (4100, 1_000));
        assert_eq!(record.kind, DumpKind::App);
    }

    // Loads the System32 debugger engine; run by hand with
    // `cargo test -p vestigium-helper -- --ignored a_truncated_dump`.
    #[test]
    #[ignore = "loads the debugger engine"]
    fn a_truncated_dump_is_kept_without_analysis() {
        let dir = tempfile::tempdir().unwrap();
        let dump = invented_dump(dir.path());
        let started = Instant::now();
        assert!(analyze(&dump.path, None, 5_000).is_none());
        assert!(started.elapsed() < Duration::from_secs(30));
    }

    #[test]
    fn refuses_dump_output_outside_dumps() {
        let good = r"C:\Users\someone\AppData\Local\Vestigium\data\dumps\1793580000000.dumps";
        assert_eq!(
            output::check_shape(Path::new(good), "dumps", "dumps"),
            Ok(())
        );
        for bad in [
            r"C:\Users\someone\AppData\Local\Vestigium\data\scans\1.dumps",
            r"C:\Users\someone\AppData\Local\Vestigium\data\dumps\1.scan",
            r"C:\Users\someone\AppData\Local\Vestigium\data\dumps\..\1.dumps",
            r"C:\Windows\Minidump\1.dumps",
        ] {
            assert!(
                output::check_shape(Path::new(bad), "dumps", "dumps").is_err(),
                "{bad}"
            );
        }
    }
}
