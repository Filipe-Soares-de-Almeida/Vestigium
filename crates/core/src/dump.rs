use serde::{Deserialize, Serialize};

use crate::Confidence;
use crate::capture::{is_intermediary, is_kernel_core};

/// Layout of the `DumpRecord` BLOB written today.
pub const DUMP_VERSION: u8 = 1;
/// Layout of the `DumpScan` file the helper writes today.
pub const DUMP_SCAN_VERSION: u8 = 1;
/// Farthest a dump's crash time may be from a fall for the two to be the same event.
pub const DUMP_FALL_REACH_MS: i64 = 5 * 60_000;

/// Where a dump came from, which says how much it can tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DumpKind {
    /// A blue screen's small dump, in the minidump folder.
    KernelMini,
    /// A blue screen's kernel or complete memory dump (MEMORY.DMP).
    KernelMemory,
    /// A report Windows wrote without stopping, such as a graphics driver that recovered.
    LiveKernel,
    /// An app that crashed.
    App,
}

/// What went wrong, as the dump records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fault {
    /// A blue screen: the stop code and its four parameters.
    BugCheck { code: u32, params: [u64; 4] },
    /// An app crash: the exception code and the address it happened at.
    Exception { code: u32, address: u64 },
}

/// What the debugger engine read from a dump; module names are image file names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Analysis {
    pub fault: Fault,
    /// The image that holds the address the fault names.
    pub fault_module: Option<String>,
    /// Images met on the faulting thread's stack, nearest first, each once.
    pub stack_modules: Vec<String>,
    /// The executable of a crashed app.
    pub process: Option<String>,
    pub module_count: u32,
}

/// A dump Windows kept, and what it says once read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DumpRecord {
    pub path: String,
    /// The file's last write, Unix epoch ms; with the path, it tells an overwritten dump apart.
    pub file_ms: i64,
    pub kind: DumpKind,
    /// When the PC or the app crashed, Unix epoch ms; the file's time when the dump does not say.
    pub crash_ms: i64,
    pub size_bytes: u64,
    /// `None` until the helper read it, and for a dump it could not read.
    pub analysis: Option<Analysis>,
}

/// What one elevated read of the dump folders found, for the watcher to import.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DumpScan {
    pub ts_ms: i64,
    pub dumps: Vec<DumpRecord>,
}

/// Who the dump points at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DumpCulprit {
    /// A driver or module named by the fault or met on the stack.
    Module {
        name: String,
        confidence: Confidence,
        intermediary: bool,
    },
    /// The processor, the memory or a device reported a fatal error.
    Hardware,
    /// The app itself, when the crash fell inside Windows' own libraries.
    App { name: String },
}

/// Stop codes whose parameters name the address that failed, and which parameter it is.
const FAULT_PARAM: [(u32, usize); 11] = [
    (0x0000_000A, 3),
    (0x0000_00D1, 3),
    (0x0000_0050, 2),
    (0x0000_001E, 1),
    (0x0000_003B, 1),
    (0x0000_007E, 1),
    (0x1000_007E, 1),
    (0x0000_008E, 1),
    (0x1000_008E, 1),
    (0x0000_0116, 1),
    (0x0000_0117, 1),
];

/// Stop codes that mean the hardware itself reported a fatal error.
const HARDWARE_STOPS: [u32; 2] = [0x0000_0124, 0x0000_009C];

/// Windows' own libraries in an app: a crash inside one is rarely theirs.
const APP_SYSTEM_IMAGES: [&str; 14] = [
    "ntdll.dll",
    "kernelbase.dll",
    "kernel32.dll",
    "ucrtbase.dll",
    "msvcrt.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "msvcp140.dll",
    "combase.dll",
    "user32.dll",
    "win32u.dll",
    "coreclr.dll",
    "clr.dll",
    "system.private.corelib.dll",
];

/// The address a fault names: the exception's, or the parameter a known stop code keeps it in.
pub fn fault_address(fault: &Fault) -> Option<u64> {
    match *fault {
        Fault::Exception { address, .. } => Some(address),
        Fault::BugCheck { code, params } => FAULT_PARAM
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, i)| params[*i])
            .filter(|a| *a != 0),
    }
}

/// The module most likely at fault: the one the fault names, else the first on the stack,
/// never the kernel; frameworks that run other drivers' code lower the confidence.
pub fn dump_culprit(analysis: &Analysis) -> Option<DumpCulprit> {
    match analysis.fault {
        Fault::BugCheck { code, .. } if HARDWARE_STOPS.contains(&code) => {
            Some(DumpCulprit::Hardware)
        }
        Fault::BugCheck { .. } => kernel_culprit(analysis),
        Fault::Exception { .. } => app_culprit(analysis),
    }
}

fn kernel_culprit(analysis: &Analysis) -> Option<DumpCulprit> {
    let named = analysis
        .fault_module
        .as_deref()
        .filter(|m| !is_kernel_core(m));
    if let Some(name) = named {
        let intermediary = is_intermediary(name);
        let behind = intermediary
            .then(|| first_driver(&analysis.stack_modules, |m| !is_intermediary(m)))
            .flatten();
        if let Some(driver) = behind {
            return Some(module(driver, Confidence::Medium, false));
        }
        let confidence = if intermediary {
            Confidence::Low
        } else {
            Confidence::High
        };
        return Some(module(name, confidence, intermediary));
    }
    if let Some(driver) = first_driver(&analysis.stack_modules, |m| !is_intermediary(m)) {
        return Some(module(driver, Confidence::Medium, false));
    }
    first_driver(&analysis.stack_modules, |_| true).map(|m| module(m, Confidence::Low, true))
}

fn app_culprit(analysis: &Analysis) -> Option<DumpCulprit> {
    let own = |m: &&String| !is_app_system(m);
    if let Some(name) = analysis.fault_module.as_ref().filter(own) {
        return Some(module(name, Confidence::High, false));
    }
    if let Some(name) = analysis.stack_modules.iter().find(own) {
        return Some(module(name, Confidence::Medium, false));
    }
    analysis
        .process
        .as_ref()
        .map(|name| DumpCulprit::App { name: name.clone() })
}

fn first_driver(stack: &[String], keep: impl Fn(&str) -> bool) -> Option<&str> {
    stack
        .iter()
        .map(String::as_str)
        .find(|m| !is_kernel_core(m) && keep(m))
}

fn module(name: &str, confidence: Confidence, intermediary: bool) -> DumpCulprit {
    DumpCulprit::Module {
        name: name.to_string(),
        confidence,
        intermediary,
    }
}

fn is_app_system(module: &str) -> bool {
    APP_SYSTEM_IMAGES
        .iter()
        .any(|n| module.eq_ignore_ascii_case(n))
}

/// The dump of a fall: a blue screen with the same stop code, written close to when it went down.
pub fn dump_of_fall(dumps: &[DumpRecord], down_ms: i64, code: u32) -> Option<&DumpRecord> {
    dumps
        .iter()
        .filter(|d| (d.crash_ms - down_ms).abs() <= DUMP_FALL_REACH_MS)
        .filter(|d| {
            d.analysis
                .as_ref()
                .is_none_or(|a| matches!(a.fault, Fault::BugCheck { code: c, .. } if c == code))
        })
        .min_by_key(|d| ((d.crash_ms - down_ms).abs(), d.kind != DumpKind::KernelMini))
}

/// The stop code and parameters of a WER-SystemErrorReporting 1001, from its first text field:
/// `0x0000000a (0xffff…, 0x0000000000000002, 0x0000000000000000, 0xfffff801…)`.
pub fn bug_check_report(text: &str) -> Option<(u32, [u64; 4])> {
    let (code, rest) = text.trim().split_once('(')?;
    let code = u32::from_str_radix(code.trim().trim_start_matches("0x"), 16).ok()?;
    let mut params = [0u64; 4];
    let mut values = rest.trim_end().trim_end_matches(')').split(',');
    for param in &mut params {
        let value = values.next()?.trim().trim_start_matches("0x");
        *param = u64::from_str_radix(value, 16).ok()?;
    }
    values.next().is_none().then_some((code, params))
}

/// What one listing changes: the dumps to add or refresh, and the ids of stored dumps whose file
/// is gone. Only paths under `covered` (folders listed, files looked up) can be found gone.
pub fn listing_changes(
    stored: &[(i64, &DumpRecord)],
    listed: &[DumpRecord],
    covered: &[String],
) -> (Vec<DumpRecord>, Vec<i64>) {
    let same = |a: &DumpRecord, b: &DumpRecord| {
        a.path.eq_ignore_ascii_case(&b.path) && a.file_ms == b.file_ms
    };
    let fresh = listed
        .iter()
        .filter(|l| {
            !stored
                .iter()
                .any(|(_, s)| same(s, l) && s.size_bytes == l.size_bytes)
        })
        .cloned()
        .collect();
    let lower = |p: &str| p.to_ascii_lowercase();
    let seen_by_listing = |path: &str| covered.iter().any(|c| lower(path).starts_with(&lower(c)));
    let gone = stored
        .iter()
        .filter(|(_, s)| seen_by_listing(&s.path) && !listed.iter().any(|l| same(s, l)))
        .map(|(id, _)| *id)
        .collect();
    (fresh, gone)
}

/// The kind of a kernel dump from where Windows keeps it.
pub fn kernel_dump_kind(path: &str) -> DumpKind {
    let lower = path.to_ascii_lowercase();
    if lower.contains("\\livekernelreports\\") {
        DumpKind::LiveKernel
    } else if lower.contains("\\minidump\\") {
        DumpKind::KernelMini
    } else {
        DumpKind::KernelMemory
    }
}
