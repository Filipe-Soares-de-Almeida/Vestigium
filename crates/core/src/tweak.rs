use serde::{Deserialize, Serialize};

use crate::health::{DelayKind, Finding, FindingKind, HealthFact, Subject, is_security};
use crate::{
    Cause, Episode, EpisodeKind, EventMeaning, OccurrenceKind, OccurrenceSummary, SHUTTING_DOWN,
    WinEvent, power_loss_detail,
};

/// Layout of the `TweakChange` BLOB written today; version 1 had no scheme in `Setting::PowerAc`.
pub const TWEAK_CHANGE_VERSION: u8 = 2;
/// A hibernation file this big is worth giving back when the system drive runs short.
pub const BIG_HIBERFILE: u64 = 1 << 30;

const DAY_MS: i64 = 24 * 60 * 60_000;
/// Longest span "did it help?" compares before and after a change.
const HELPED_DAYS: i64 = 7;
/// FILETIME counts 100 ns from 1601; Unix milliseconds count from 1970.
const FILETIME_UNIX_MS: i64 = 11_644_473_600_000;

pub const SCHEME_BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
pub const SCHEME_POWER_SAVER: Guid = Guid(0xa1841308_3541_4fab_bc81_f71556f20b4a);
pub const SCHEME_HIGH_PERFORMANCE: Guid = Guid(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
pub const SUB_PCIEXPRESS: Guid = Guid(0x501a4d13_42af_4429_9fd1_a8218c268e20);
pub const PCIE_ASPM: Guid = Guid(0xee12f906_d277_404b_b6da_e5fa1a576df5);
pub const SUB_USB: Guid = Guid(0x2a737441_1930_4402_8d77_b2bebba308a3);
pub const USB_SELECTIVE_SUSPEND: Guid = Guid(0x48e6b7a6_50f5_4782_a5d4_53bb8f07e226);

const POWER_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Power";
const GRAPHICS_KEY: &str = r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers";
const DWM_KEY: &str = r"SOFTWARE\Microsoft\Windows\Dwm";
const CRASH_KEY: &str = r"SYSTEM\CurrentControlSet\Control\CrashControl";
const LOCAL_DUMPS_KEY: &str = r"SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps";
const GAME_DVR_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
const GAME_CONFIG_KEY: &str = r"System\GameConfigStore";

/// A GUID such as a power scheme's, kept as text so JSON and the window read it whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Guid(pub u128);

impl std::fmt::Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0;
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            v >> 96,
            (v >> 80) & 0xffff,
            (v >> 64) & 0xffff,
            (v >> 48) & 0xffff,
            v & 0xffff_ffff_ffff
        )
    }
}

impl std::str::FromStr for Guid {
    type Err = String;

    fn from_str(text: &str) -> Result<Guid, String> {
        let digits: String = text.trim_matches(['{', '}']).split('-').collect();
        if digits.len() != 32 {
            return Err(format!("{text} is not a GUID"));
        }
        u128::from_str_radix(&digits, 16)
            .map(Guid)
            .map_err(|e| e.to_string())
    }
}

impl Serialize for Guid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Guid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Guid, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Services that are never changed: security, updates and what keeps them running.
const FORBIDDEN_SERVICES: [&str; 12] = [
    "windefend",
    "wdnissvc",
    "sense",
    "securityhealthservice",
    "wscsvc",
    "mpssvc",
    "bfe",
    "wuauserv",
    "usosvc",
    "waasmedicsvc",
    "bits",
    "appidsvc",
];
/// Registry places that are never written: security, updates, UAC, mitigations and the page file.
const FORBIDDEN_KEYS: [&str; 10] = [
    "windows defender",
    "deviceguard",
    "memory management",
    r"policies\system",
    "sharedaccess",
    "windowsupdate",
    "smartscreen",
    r"control\lsa",
    "hypervisorenforcedcodeintegrity",
    r"session manager\kernel",
];

/// One Windows setting Vestigium may offer to change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TweakId {
    FastStartup,
    Hibernation,
    PowerPlan,
    PcieAspm,
    UsbSuspend,
    SysMain,
    WindowsSearch,
    GameDvr,
    GpuScheduling,
    Mpo,
    StartupApp,
    AppDumps,
    CrashDump,
}

pub const TWEAKS: [TweakId; 13] = [
    TweakId::FastStartup,
    TweakId::Hibernation,
    TweakId::PowerPlan,
    TweakId::PcieAspm,
    TweakId::UsbSuspend,
    TweakId::SysMain,
    TweakId::WindowsSearch,
    TweakId::GameDvr,
    TweakId::GpuScheduling,
    TweakId::Mpo,
    TweakId::StartupApp,
    TweakId::AppDumps,
    TweakId::CrashDump,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hive {
    LocalMachine,
    CurrentUser,
}

/// The list of Explorer's startup approvals an entry belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StartupList {
    Run,
    Run32,
    StartupFolder,
}

impl StartupList {
    /// Where Explorer keeps the approvals of this list.
    pub fn approval_key(self) -> &'static str {
        match self {
            StartupList::Run => {
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
            }
            StartupList::Run32 => {
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32"
            }
            StartupList::StartupFolder => {
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder"
            }
        }
    }
}

/// Something Windows keeps that a tweak reads and writes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Setting {
    /// A DWORD registry value.
    Dword {
        hive: Hive,
        key: String,
        name: String,
    },
    /// A registry key that may or may not exist; it is deleted only when empty.
    Key {
        hive: Hive,
        key: String,
    },
    /// A service's start type and whether it runs.
    Service {
        name: String,
    },
    /// A plugged-in value of a power scheme: `scheme` when known, the active one when `None`.
    PowerAc {
        subgroup: Guid,
        setting: Guid,
        #[serde(default)]
        scheme: Option<Guid>,
    },
    ActiveScheme,
    /// Whether Windows keeps a hibernation file.
    Hibernation,
    /// Explorer's approval of one startup entry.
    StartupApproval {
        hive: Hive,
        list: StartupList,
        name: String,
    },
}

/// What a setting holds; `Absent` is a registry value, key or approval that does not exist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
    Absent,
    Dword(u32),
    Present,
    /// Start type 2 automatic, 3 manual, 4 disabled.
    Service {
        start: u32,
        delayed: bool,
        running: bool,
    },
    Index(u32),
    Scheme(Guid),
    On(bool),
    Bytes(Vec<u8>),
}

impl Setting {
    fn dword(hive: Hive, key: &str, name: &str) -> Setting {
        Setting::Dword {
            hive,
            key: key.to_string(),
            name: name.to_string(),
        }
    }

    fn service(name: &str) -> Setting {
        Setting::Service {
            name: name.to_string(),
        }
    }

    /// The setting with the scheme of a plugged-in power value left out, as the catalog lists it.
    fn without_scheme(&self) -> Setting {
        match self {
            Setting::PowerAc {
                subgroup, setting, ..
            } => Setting::PowerAc {
                subgroup: *subgroup,
                setting: *setting,
                scheme: None,
            },
            other => other.clone(),
        }
    }

    /// Who may write it: the watcher as the signed-in user, or the elevated helper.
    pub fn applier(&self) -> Applier {
        match self {
            Setting::Dword { hive, .. }
            | Setting::Key { hive, .. }
            | Setting::StartupApproval { hive, .. } => match hive {
                Hive::CurrentUser => Applier::User,
                Hive::LocalMachine => Applier::Elevated,
            },
            Setting::PowerAc { .. } | Setting::ActiveScheme => Applier::User,
            Setting::Service { .. } | Setting::Hibernation => Applier::Elevated,
        }
    }

    /// A setting no tweak may ever write: security, updates, UAC, mitigations, the page file.
    pub fn is_forbidden(&self) -> bool {
        match self {
            Setting::Dword { key, .. } | Setting::Key { key, .. } => {
                let key = key.to_lowercase();
                FORBIDDEN_KEYS.iter().any(|k| key.contains(k))
            }
            Setting::Service { name } => FORBIDDEN_SERVICES.contains(&name.to_lowercase().as_str()),
            Setting::StartupApproval { name, .. } => is_security(name),
            Setting::PowerAc { .. } | Setting::ActiveScheme | Setting::Hibernation => false,
        }
    }

    /// Whether a value has the shape this setting holds, so that restoring it stays in its domain.
    pub fn fits(&self, value: &Value) -> bool {
        match (self, value) {
            (Setting::Dword { .. }, Value::Absent)
            | (Setting::Key { .. }, Value::Present | Value::Absent)
            | (Setting::PowerAc { .. }, Value::Index(_))
            | (Setting::ActiveScheme, Value::Scheme(_))
            | (Setting::Hibernation, Value::On(_))
            | (Setting::StartupApproval { .. }, Value::Absent) => true,
            (Setting::Dword { name, .. }, Value::Dword(v)) => {
                dword_range(name).is_some_and(|range| range.contains(v))
            }
            (Setting::Service { .. }, Value::Service { start, .. }) => (2..=4).contains(start),
            (Setting::StartupApproval { .. }, Value::Bytes(b)) => b.len() == 12 && b[0] < 8,
            _ => false,
        }
    }
}

/// The values a DWORD of the catalog takes, wide enough for anything a person may have set by hand.
/// HiberbootEnabled, the Game DVR flags: 0 or 1. DumpType: 0 custom, 1 mini, 2 full. CrashDumpEnabled:
/// 0 to 7 (none, complete, kernel, small, automatic). HwSchMode: 1 off, 2 on. DumpCount has no
/// documented maximum and OverlayTestMode is undocumented, so each gets a generous cap.
fn dword_range(name: &str) -> Option<std::ops::RangeInclusive<u32>> {
    Some(match name {
        "HiberbootEnabled"
        | "AppCaptureEnabled"
        | "HistoricalCaptureEnabled"
        | "GameDVR_Enabled" => 0..=1,
        "DumpType" | "HwSchMode" => 0..=2,
        "CrashDumpEnabled" => 0..=7,
        "OverlayTestMode" => 0..=255,
        "DumpCount" => 0..=10_000,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Applier {
    User,
    Elevated,
}

impl Applier {
    /// Every variant, in declaration order.
    pub const ALL: [Applier; 2] = [Applier::User, Applier::Elevated];
}

/// When a change starts to count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Takes {
    Now,
    NextShutdown,
    NextSignIn,
    Restart,
}

impl Takes {
    /// Every variant, in declaration order.
    pub const ALL: [Takes; 4] = [
        Takes::Now,
        Takes::NextShutdown,
        Takes::NextSignIn,
        Takes::Restart,
    ];
}

impl TweakId {
    pub fn takes(self) -> Takes {
        match self {
            TweakId::FastStartup => Takes::NextShutdown,
            TweakId::StartupApp => Takes::NextSignIn,
            TweakId::GpuScheduling | TweakId::Mpo | TweakId::CrashDump => Takes::Restart,
            _ => Takes::Now,
        }
    }

    /// The fixed settings this tweak reads; a startup entry's approval is added per entry.
    pub fn settings(self) -> Vec<Setting> {
        use Hive::{CurrentUser, LocalMachine};
        match self {
            TweakId::FastStartup => {
                vec![Setting::dword(LocalMachine, POWER_KEY, "HiberbootEnabled")]
            }
            TweakId::Hibernation => vec![Setting::Hibernation],
            TweakId::PowerPlan => vec![Setting::ActiveScheme],
            TweakId::PcieAspm => vec![Setting::PowerAc {
                subgroup: SUB_PCIEXPRESS,
                setting: PCIE_ASPM,
                scheme: None,
            }],
            TweakId::UsbSuspend => vec![Setting::PowerAc {
                subgroup: SUB_USB,
                setting: USB_SELECTIVE_SUSPEND,
                scheme: None,
            }],
            TweakId::SysMain => vec![Setting::service("SysMain")],
            TweakId::WindowsSearch => vec![Setting::service("WSearch")],
            TweakId::GameDvr => vec![
                Setting::dword(CurrentUser, GAME_DVR_KEY, "AppCaptureEnabled"),
                Setting::dword(CurrentUser, GAME_DVR_KEY, "HistoricalCaptureEnabled"),
                Setting::dword(CurrentUser, GAME_CONFIG_KEY, "GameDVR_Enabled"),
            ],
            TweakId::GpuScheduling => vec![Setting::dword(LocalMachine, GRAPHICS_KEY, "HwSchMode")],
            TweakId::Mpo => vec![Setting::dword(LocalMachine, DWM_KEY, "OverlayTestMode")],
            TweakId::StartupApp => Vec::new(),
            TweakId::AppDumps => vec![
                Setting::Key {
                    hive: LocalMachine,
                    key: LOCAL_DUMPS_KEY.to_string(),
                },
                Setting::dword(LocalMachine, LOCAL_DUMPS_KEY, "DumpType"),
                Setting::dword(LocalMachine, LOCAL_DUMPS_KEY, "DumpCount"),
            ],
            TweakId::CrashDump => vec![Setting::dword(LocalMachine, CRASH_KEY, "CrashDumpEnabled")],
        }
    }
}

/// One setting moved from one value to another; undoing swaps them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingChange {
    pub setting: Setting,
    pub from: Value,
    pub to: Value,
}

/// The changes that undo `changes`, in reverse order.
pub fn undo(changes: &[SettingChange]) -> Vec<SettingChange> {
    changes
        .iter()
        .rev()
        .map(|c| SettingChange {
            setting: c.setting.clone(),
            from: c.to.clone(),
            to: c.from.clone(),
        })
        .collect()
}

/// An entry Windows starts at sign-in, from a Run key or a Startup folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupEntry {
    pub hive: Hive,
    pub list: StartupList,
    pub name: String,
    /// The program's file name, such as "Chat.exe".
    pub exe: String,
}

impl StartupEntry {
    pub fn approval(&self) -> Setting {
        Setting::StartupApproval {
            hive: self.hive,
            list: self.list,
            name: self.name.clone(),
        }
    }
}

/// What a deep capture named: drivers and apps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaptureCauses {
    pub start_ms: i64,
    pub causes: Vec<Cause>,
}

/// Everything the tweak rules look at: what each setting holds now and what Vestigium observed.
#[derive(Clone, Debug, PartialEq)]
pub struct TweakInputs {
    pub now_ms: i64,
    /// Oldest time the occurrences, episodes and events below reach back to.
    pub evidence_from_ms: i64,
    /// What each setting holds, read without elevation; a setting missing here could not be read.
    pub values: Vec<(Setting, Value)>,
    pub startup: Vec<StartupEntry>,
    pub hiberfile_bytes: Option<u64>,
    /// Root of the volume that holds Windows, such as "C:\".
    pub system_root: String,
    /// Instance name of the physical disk that holds Windows, such as "0 C:".
    pub system_disk: Option<String>,
    pub monitors: u32,
    pub battery: bool,
    /// Health findings open now.
    pub findings: Vec<Finding>,
    pub occurrences: Vec<OccurrenceSummary>,
    pub episodes: Vec<Episode>,
    pub captures: Vec<CaptureCauses>,
    pub events: Vec<WinEvent>,
}

/// One thing Vestigium observed that backs a tweak; the UI words it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Signal {
    Finding(FindingKind),
    UnfinishedShutdowns { count: u32 },
    SleepFalls { count: u32 },
    Stutters { count: u32 },
    DpcIsrStalls { count: u32 },
    GpuDriverStalls { count: u32 },
    DiskBusy { count: u32, culprit: Option<String> },
    CpuSaturated { count: u32, culprit: Option<String> },
    CaptureDriver { start_ms: i64, module: String },
    CaptureFrames { start_ms: i64, app: String },
    AppCrashes { count: u32 },
    HiberFile { bytes: u64 },
    Monitors { count: u32 },
    DumpsOff,
    SlowStartup { name: String, total_ms: u32 },
}

/// Whether a tweak can be offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Ready,
    /// Windows already holds what the tweak would write.
    AlreadySet,
    /// The setting could not be read or does not exist on this PC.
    Unavailable,
    /// A security product's startup entry, never offered.
    Protected,
}

/// One tweak as the page shows it: the change it would make and the signals behind it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Advice {
    pub tweak: TweakId,
    /// The startup entry's name, for `StartupApp`.
    pub subject: Option<String>,
    pub status: Status,
    /// Empty when nothing here says the tweak would help.
    pub signals: Vec<Signal>,
    pub changes: Vec<SettingChange>,
    /// What each setting of the tweak holds now; empty when none could be read.
    pub current: Vec<SettingValue>,
    pub applier: Applier,
    pub takes: Takes,
}

/// One setting and what it holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingValue {
    pub setting: Setting,
    pub value: Value,
}

impl Advice {
    pub fn recommended(&self) -> bool {
        self.status == Status::Ready && !self.signals.is_empty()
    }
}

/// Every tweak with its state and signals, recommended ones first.
pub fn advise(inputs: &TweakInputs) -> Vec<Advice> {
    let mut out: Vec<Advice> = TWEAKS
        .iter()
        .filter(|t| **t != TweakId::StartupApp)
        .map(|&tweak| {
            let changes = target(tweak, inputs);
            let status = match &changes {
                None => Status::Unavailable,
                Some(c) if c.is_empty() => Status::AlreadySet,
                Some(_) => Status::Ready,
            };
            advice(
                tweak,
                None,
                status,
                signals(tweak, inputs),
                changes.unwrap_or_default(),
                &tweak.settings(),
                inputs,
            )
        })
        .collect();
    for entry in &inputs.startup {
        out.push(startup_advice(entry, inputs));
    }
    out.sort_by_key(|a| (!a.recommended(), a.status != Status::Ready));
    out
}

fn advice(
    tweak: TweakId,
    subject: Option<String>,
    status: Status,
    signals: Vec<Signal>,
    changes: Vec<SettingChange>,
    settings: &[Setting],
    inputs: &TweakInputs,
) -> Advice {
    let applier = if !settings.is_empty() && settings.iter().all(|s| s.applier() == Applier::User) {
        Applier::User
    } else {
        Applier::Elevated
    };
    Advice {
        tweak,
        subject,
        status,
        signals,
        changes,
        current: settings
            .iter()
            .filter_map(|s| {
                value(inputs, s).map(|v| SettingValue {
                    setting: s.clone(),
                    value: v.clone(),
                })
            })
            .collect(),
        applier,
        takes: tweak.takes(),
    }
}

fn value<'a>(inputs: &'a TweakInputs, setting: &Setting) -> Option<&'a Value> {
    inputs
        .values
        .iter()
        .find(|(s, _)| s == setting)
        .map(|(_, v)| v)
}

/// The changes a tweak would make now: `None` when it cannot be read, empty when already set.
fn target(tweak: TweakId, inputs: &TweakInputs) -> Option<Vec<SettingChange>> {
    let mut changes = Vec::new();
    for setting in tweak.settings() {
        let from = value(inputs, &setting)?.clone();
        // A LocalDumps key already there was set up by someone; it is left as it is.
        if tweak == TweakId::AppDumps && from == Value::Present {
            return Some(Vec::new());
        }
        let to = apply_value(tweak, &setting, &from)?;
        if from != to {
            changes.push(SettingChange { setting, from, to });
        }
    }
    Some(changes)
}

/// What applying `tweak` writes to `setting` while it holds `from`; `None` when the tweak
/// cannot act on it. A startup approval is written by `disabled_approval` instead.
pub fn apply_value(tweak: TweakId, setting: &Setting, from: &Value) -> Option<Value> {
    Some(match (tweak, setting, from) {
        (TweakId::StartupApp, _, _) => return None,
        (TweakId::FastStartup | TweakId::GpuScheduling, _, Value::Absent) => return None,
        (TweakId::GpuScheduling, _, Value::Dword(2)) => Value::Dword(1),
        (TweakId::GpuScheduling, _, Value::Dword(1)) => Value::Dword(2),
        (TweakId::GpuScheduling, _, _) => return None,
        (TweakId::PowerPlan, _, Value::Scheme(SCHEME_POWER_SAVER)) => {
            Value::Scheme(SCHEME_BALANCED)
        }
        (TweakId::CrashDump, _, Value::Dword(0)) => Value::Dword(7),
        (TweakId::PowerPlan | TweakId::CrashDump, _, _) => from.clone(),
        (TweakId::AppDumps, Setting::Key { .. }, _) => Value::Present,
        (TweakId::AppDumps, Setting::Dword { name, .. }, _) if name == "DumpType" => {
            Value::Dword(1)
        }
        (TweakId::AppDumps, _, _) => Value::Dword(10),
        (TweakId::PcieAspm | TweakId::UsbSuspend, _, _) => Value::Index(0),
        (TweakId::Hibernation, _, _) => Value::On(false),
        (TweakId::SysMain | TweakId::WindowsSearch, _, _) => Value::Service {
            start: 4,
            delayed: false,
            running: false,
        },
        (TweakId::Mpo, _, _) => Value::Dword(5),
        (TweakId::FastStartup | TweakId::GameDvr, _, _) => Value::Dword(0),
    })
}

fn startup_advice(entry: &StartupEntry, inputs: &TweakInputs) -> Advice {
    let setting = entry.approval();
    let settings = [setting.clone()];
    let from = value(inputs, &setting).cloned().unwrap_or(Value::Absent);
    let disabled = matches!(&from, Value::Bytes(b) if b.first().is_some_and(|f| f & 1 == 1));
    let status = if is_security(&entry.name) || is_security(&entry.exe) {
        Status::Protected
    } else if disabled {
        Status::AlreadySet
    } else {
        Status::Ready
    };
    let changes = if status == Status::Ready {
        vec![SettingChange {
            setting,
            from,
            to: Value::Bytes(disabled_approval(inputs.now_ms)),
        }]
    } else {
        Vec::new()
    };
    let signals = if status == Status::Protected {
        Vec::new()
    } else {
        slow_startup(entry, inputs)
    };
    advice(
        TweakId::StartupApp,
        Some(entry.name.clone()),
        status,
        signals,
        changes,
        &settings,
        inputs,
    )
}

/// The approval Explorer writes for a disabled entry: 03, three zero bytes and when it was disabled.
pub fn disabled_approval(now_ms: i64) -> Vec<u8> {
    let filetime = (now_ms + FILETIME_UNIX_MS).max(0) as u64 * 10_000;
    let mut bytes = vec![3, 0, 0, 0];
    bytes.extend_from_slice(&filetime.to_le_bytes());
    bytes
}

fn slow_startup(entry: &StartupEntry, inputs: &TweakInputs) -> Vec<Signal> {
    let stem = entry
        .exe
        .to_lowercase()
        .trim_end_matches(".exe")
        .to_string();
    let name = entry.name.to_lowercase();
    inputs
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::SlowBoot)
        .flat_map(|f| &f.evidence)
        .filter_map(|fact| match fact {
            HealthFact::Delay(d) if d.kind == DelayKind::App && !d.security => Some(d),
            _ => None,
        })
        .filter(|d| {
            let delay = d.name.to_lowercase();
            same_app(&delay, &stem) || same_app(&delay, &name)
        })
        .map(|d| Signal::SlowStartup {
            name: d.name.clone(),
            total_ms: d.total_ms,
        })
        .take(1)
        .collect()
}

/// Two names of one program: equal once reduced to lowercase letters and digits, without `.exe`.
fn same_app(a: &str, b: &str) -> bool {
    let reduce = |name: &str| {
        name.to_lowercase()
            .trim_end_matches(".exe")
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    };
    let (a, b) = (reduce(a), reduce(b));
    !a.is_empty() && a == b
}

fn since(inputs: &TweakInputs, days: i64) -> i64 {
    inputs.now_ms - days * DAY_MS
}

fn has_finding(inputs: &TweakInputs, kind: FindingKind) -> bool {
    inputs.findings.iter().any(|f| f.kind == kind)
}

fn count_occurrences(
    inputs: &TweakInputs,
    from: i64,
    to: i64,
    pick: impl Fn(&OccurrenceSummary) -> bool,
) -> u32 {
    inputs
        .occurrences
        .iter()
        .filter(|o| o.start_ms >= from && o.start_ms < to && pick(o))
        .count() as u32
}

fn stutters(inputs: &TweakInputs, from: i64, to: i64) -> u32 {
    count_occurrences(inputs, from, to, |o| o.kind == OccurrenceKind::Stutter)
}

fn unfinished_shutdowns(inputs: &TweakInputs, from: i64, to: i64) -> u32 {
    inputs
        .events
        .iter()
        .filter(|e| e.ts_ms >= from && e.ts_ms < to && e.meaning == EventMeaning::PowerLoss)
        .filter(|e| power_loss_detail(&e.data_xml).is_some_and(|d| d.power_state == SHUTTING_DOWN))
        .count() as u32
}

fn events(inputs: &TweakInputs, from: i64, to: i64, meaning: EventMeaning) -> u32 {
    inputs
        .events
        .iter()
        .filter(|e| e.ts_ms >= from && e.ts_ms < to && e.meaning == meaning)
        .count() as u32
}

/// Episodes that started in `[from, to)` and match, with the program most often blamed.
fn episodes(
    inputs: &TweakInputs,
    from: i64,
    to: i64,
    pick: impl Fn(&Episode) -> bool,
) -> (u32, Option<String>) {
    let picked: Vec<&Episode> = inputs
        .episodes
        .iter()
        .filter(|e| e.start_ms >= from && e.start_ms < to && pick(e))
        .collect();
    let mut tally: Vec<(&str, usize)> = Vec::new();
    for name in picked
        .iter()
        .filter_map(|e| e.culprit.as_ref())
        .map(|c| c.name.as_str())
    {
        match tally.iter_mut().find(|(n, _)| *n == name) {
            Some((_, count)) => *count += 1,
            None => tally.push((name, 1)),
        }
    }
    let culprit = tally
        .iter()
        .max_by_key(|(_, n)| *n)
        .map(|(name, _)| name.to_string());
    (picked.len() as u32, culprit)
}

const SEARCH_PROCESSES: [&str; 3] = [
    "searchindexer.exe",
    "searchprotocolhost.exe",
    "searchfilterhost.exe",
];

fn is_search(e: &Episode) -> bool {
    e.culprit
        .as_ref()
        .is_some_and(|c| SEARCH_PROCESSES.contains(&c.name.to_lowercase().as_str()))
}

fn on_system_disk(inputs: &TweakInputs, e: &Episode) -> bool {
    matches!(&e.kind, EpisodeKind::DiskBusy { disk } if inputs.system_disk.as_ref() == Some(disk))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bus {
    Storage,
    Network,
    Usb,
}

/// The bus a kernel module drives, for the power savings that act on it.
fn bus_of(module: &str) -> Option<Bus> {
    let m = module.to_lowercase();
    let stem = m.trim_end_matches(".sys");
    let any = |names: &[&str]| {
        names
            .iter()
            .any(|n| stem == *n || (n.ends_with('*') && stem.starts_with(n.trim_end_matches('*'))))
    };
    if any(&[
        "storport", "stornvme", "storahci", "ataport", "iastor*", "nvme*",
    ]) {
        Some(Bus::Storage)
    } else if any(&[
        "ndis", "tcpip", "netio", "netwtw*", "netwbw*", "rt640x64", "rtwlane*", "e1*", "athw*",
        "bcmwl*",
    ]) {
        Some(Bus::Network)
    } else if any(&[
        "usbxhci", "usbhub3", "ucx01000", "usbport", "usbhub", "usbccgp",
    ]) {
        Some(Bus::Usb)
    } else {
        None
    }
}

fn capture_drivers(inputs: &TweakInputs, days: i64, buses: &[Bus]) -> Vec<Signal> {
    inputs
        .captures
        .iter()
        .filter(|c| c.start_ms >= since(inputs, days))
        .flat_map(|c| c.causes.iter().map(move |cause| (c.start_ms, cause)))
        .filter_map(|(start_ms, cause)| match cause {
            Cause::DriverModule { module, .. }
                if bus_of(module).is_some_and(|b| buses.contains(&b)) =>
            {
                Some(Signal::CaptureDriver {
                    start_ms,
                    module: module.clone(),
                })
            }
            _ => None,
        })
        .take(1)
        .collect()
}

/// The signals that back a tweak on this PC; empty when nothing here says it would help.
fn signals(tweak: TweakId, inputs: &TweakInputs) -> Vec<Signal> {
    let now = inputs.now_ms + 1;
    let week = since(inputs, 7);
    let mut out = Vec::new();
    match tweak {
        TweakId::FastStartup => {
            if has_finding(inputs, FindingKind::ShutdownUnfinished) {
                out.push(Signal::Finding(FindingKind::ShutdownUnfinished));
                let count = unfinished_shutdowns(inputs, since(inputs, 30), now);
                if count > 0 {
                    out.push(Signal::UnfinishedShutdowns { count });
                }
            }
            let falls = count_occurrences(inputs, since(inputs, 30), now, |o| {
                o.kind == OccurrenceKind::PostMortem && o.cause == Cause::SleepTransition
            });
            if falls > 0 {
                out.push(Signal::SleepFalls { count: falls });
            }
        }
        TweakId::Hibernation => {
            let low = inputs.findings.iter().any(|f| {
                f.kind == FindingKind::LowSpace
                    && matches!(&f.subject, Subject::Volume(root) if root.eq_ignore_ascii_case(&inputs.system_root))
            });
            if let Some(bytes) = inputs.hiberfile_bytes.filter(|b| *b >= BIG_HIBERFILE)
                && low
            {
                out.push(Signal::Finding(FindingKind::LowSpace));
                out.push(Signal::HiberFile { bytes });
            }
        }
        TweakId::PowerPlan => {
            if inputs.battery
                || value(inputs, &Setting::ActiveScheme) != Some(&Value::Scheme(SCHEME_POWER_SAVER))
            {
                return out;
            }
            let count = stutters(inputs, week, now);
            if count >= 3 {
                out.push(Signal::Stutters { count });
            }
            let (count, culprit) =
                episodes(inputs, week, now, |e| e.kind == EpisodeKind::CpuSaturated);
            if count >= 2 {
                out.push(Signal::CpuSaturated { count, culprit });
            }
            if has_finding(inputs, FindingKind::Throttling) {
                out.push(Signal::Finding(FindingKind::Throttling));
            }
        }
        TweakId::PcieAspm => {
            let count = count_occurrences(inputs, since(inputs, 14), now, |o| {
                matches!(o.cause, Cause::Driver { .. })
            });
            if count >= 2 {
                out.push(Signal::DpcIsrStalls { count });
            }
            out.extend(capture_drivers(inputs, 14, &[Bus::Storage, Bus::Network]));
        }
        TweakId::UsbSuspend => out.extend(capture_drivers(inputs, 14, &[Bus::Usb])),
        TweakId::SysMain => {
            let (count, culprit) = episodes(inputs, week, now, |e| on_system_disk(inputs, e));
            if has_finding(inputs, FindingKind::WindowsOnHdd) && count >= 2 {
                out.push(Signal::Finding(FindingKind::WindowsOnHdd));
                out.push(Signal::DiskBusy { count, culprit });
            }
        }
        TweakId::WindowsSearch => {
            let from = since(inputs, 14);
            let (disk, disk_culprit) = episodes(inputs, from, now, |e| {
                matches!(e.kind, EpisodeKind::DiskBusy { .. }) && is_search(e)
            });
            let (cpu, cpu_culprit) = episodes(inputs, from, now, |e| {
                e.kind == EpisodeKind::CpuSaturated && is_search(e)
            });
            if disk + cpu >= 2 {
                if disk > 0 {
                    out.push(Signal::DiskBusy {
                        count: disk,
                        culprit: disk_culprit,
                    });
                }
                if cpu > 0 {
                    out.push(Signal::CpuSaturated {
                        count: cpu,
                        culprit: cpu_culprit,
                    });
                }
            }
        }
        TweakId::GameDvr => {
            let historical = &TweakId::GameDvr.settings()[1];
            if value(inputs, historical) != Some(&Value::Dword(1)) {
                return out;
            }
            let count = stutters(inputs, week, now);
            if count >= 3 {
                out.push(Signal::Stutters { count });
            }
            let frames = inputs
                .captures
                .iter()
                .filter(|c| c.start_ms >= since(inputs, 14))
                .flat_map(|c| c.causes.iter().map(move |cause| (c.start_ms, cause)))
                .find_map(|(start_ms, cause)| match cause {
                    Cause::Frames { app } => Some(Signal::CaptureFrames {
                        start_ms,
                        app: app.clone(),
                    }),
                    _ => None,
                });
            out.extend(frames);
        }
        TweakId::GpuScheduling => {
            if has_finding(inputs, FindingKind::DisplayResets) {
                out.push(Signal::Finding(FindingKind::DisplayResets));
            }
            let count = count_occurrences(inputs, since(inputs, 30), now, |o| {
                o.cause == Cause::GpuDriver
            });
            if count >= 2 {
                out.push(Signal::GpuDriverStalls { count });
            }
        }
        TweakId::Mpo => {
            if inputs.monitors >= 2 {
                let mut backing = Vec::new();
                if has_finding(inputs, FindingKind::DisplayResets) {
                    backing.push(Signal::Finding(FindingKind::DisplayResets));
                }
                let count = stutters(inputs, week, now);
                if count >= 3 {
                    backing.push(Signal::Stutters { count });
                }
                if !backing.is_empty() {
                    out.push(Signal::Monitors {
                        count: inputs.monitors,
                    });
                    out.extend(backing);
                }
            }
        }
        TweakId::StartupApp => {}
        TweakId::AppDumps => {
            let count = events(inputs, since(inputs, 30), now, EventMeaning::AppCrash);
            if count > 0 {
                out.push(Signal::AppCrashes { count });
            }
        }
        TweakId::CrashDump => {
            let off = Setting::dword(Hive::LocalMachine, CRASH_KEY, "CrashDumpEnabled");
            if value(inputs, &off) == Some(&Value::Dword(0)) {
                out.push(Signal::DumpsOff);
            }
        }
    }
    out
}

/// The same signal counted in equal spans before and after a change, at most a week each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Helped {
    pub days: u32,
    pub before: u32,
    pub after: u32,
}

/// Whether a change helped, by the signal that backs its tweak; `None` for a tweak with nothing to
/// count, a change less than a day old, or a before window that starts earlier than the evidence.
pub fn helped(tweak: TweakId, applied_ms: i64, inputs: &TweakInputs) -> Option<Helped> {
    let days = ((inputs.now_ms - applied_ms) / DAY_MS).min(HELPED_DAYS);
    if days < 1 {
        return None;
    }
    let span = days * DAY_MS;
    if applied_ms - span < inputs.evidence_from_ms {
        return None;
    }
    let count = |from: i64, to: i64| -> Option<u32> {
        Some(match tweak {
            TweakId::FastStartup => unfinished_shutdowns(inputs, from, to),
            TweakId::PowerPlan
            | TweakId::PcieAspm
            | TweakId::UsbSuspend
            | TweakId::GameDvr
            | TweakId::Mpo => stutters(inputs, from, to),
            TweakId::SysMain => episodes(inputs, from, to, |e| on_system_disk(inputs, e)).0,
            TweakId::WindowsSearch => episodes(inputs, from, to, is_search).0,
            TweakId::GpuScheduling => events(inputs, from, to, EventMeaning::DisplayReset),
            TweakId::Hibernation | TweakId::StartupApp | TweakId::AppDumps | TweakId::CrashDump => {
                return None;
            }
        })
    };
    Some(Helped {
        days: days as u32,
        before: count(applied_ms - span, applied_ms)?,
        after: count(applied_ms, applied_ms + span)?,
    })
}

/// What writing one setting did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingResult {
    pub change: SettingChange,
    /// The Win32 error when the write failed or was refused.
    pub error: Option<u32>,
}

/// What became of the restore point asked for before an elevated batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestorePoint {
    Created,
    /// Windows skipped it: a point from `at_ms` is recent enough.
    Reused {
        at_ms: Option<i64>,
    },
    /// System Restore is turned off.
    Off,
    Failed {
        error: u32,
    },
    /// Windows accepted the request but logged nothing about it in time.
    Unconfirmed,
}

/// One change to Windows as the history keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakChange {
    pub tweak: TweakId,
    pub subject: Option<String>,
    /// The history row this change undoes.
    pub undo_of: Option<i64>,
    pub results: Vec<SettingResult>,
    pub restore_point: Option<RestorePoint>,
}

impl TweakChange {
    pub fn succeeded(&self) -> bool {
        self.results
            .iter()
            .all(|r| r.error.is_none() || r.error == Some(ERROR_INTERRUPTED_WRITTEN))
    }

    /// The changes Windows holds or may hold, which an undo puts back: the writes that succeeded,
    /// those found in place after an interrupted helper and those that failed with
    /// `ERROR_MAY_HAVE_CHANGED`, leaving out what did not move.
    pub fn written(&self) -> Vec<SettingChange> {
        self.results
            .iter()
            .filter(|r| {
                r.error.is_none()
                    || r.error == Some(ERROR_MAY_HAVE_CHANGED)
                    || r.error == Some(ERROR_INTERRUPTED_WRITTEN)
            })
            .filter(|r| r.change.from != r.change.to)
            .map(|r| r.change.clone())
            .collect()
    }
}

/// Layout of the `TweakPlan` the window writes and the `TweakResults` the helper writes today.
pub const TWEAK_PLAN_VERSION: u8 = 1;
/// Most settings one item may change.
const MOST_CHANGES: usize = 8;
/// Win32 errors a change records when it is not written.
pub const ERROR_UNREADABLE: u32 = 1168;
pub const ERROR_CHANGED_SINCE: u32 = 13;
/// A write failed and putting the old value back failed too, so the setting may hold either.
pub const ERROR_MAY_HAVE_CHANGED: u32 = 0x2000_0001;
/// The helper stopped before it reported, and Windows holds the value it was to write.
pub const ERROR_INTERRUPTED_WRITTEN: u32 = 0x2000_0002;
/// The helper stopped before it reported, and Windows still holds the value from before.
pub const ERROR_INTERRUPTED_UNCHANGED: u32 = 0x2000_0003;
/// The helper stopped before it reported, and Windows holds neither value, or it cannot be read.
pub const ERROR_INTERRUPTED_UNKNOWN: u32 = 0x2000_0004;
/// Layout of the `TweakIntent` the helper writes before its first change.
pub const TWEAK_INTENT_VERSION: u8 = 1;

/// One tweak to apply or undo, as the window asks for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanItem {
    pub tweak: TweakId,
    pub subject: Option<String>,
    /// The history row this item undoes; its changes then carry the values to put back.
    pub undo_of: Option<i64>,
    pub changes: Vec<SettingChange>,
}

/// What the window asks the elevated helper to change, in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakPlan {
    pub items: Vec<PlanItem>,
    /// The person agreed to go on when Windows makes no restore point.
    pub without_restore_point: bool,
}

/// What the helper did with a plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakResults {
    pub ts_ms: i64,
    pub changes: Vec<TweakChange>,
}

/// Whether `applier` may carry out `item`: every setting belongs to the tweak, is written by that
/// applier, is not forbidden, and gets a value of its own shape.
pub fn check_item(item: &PlanItem, applier: Applier) -> Result<(), String> {
    if item.changes.is_empty() || item.changes.len() > MOST_CHANGES {
        return Err(format!("{} changes", item.changes.len()));
    }
    let catalog = item.tweak.settings();
    for change in &item.changes {
        let setting = &change.setting;
        let belongs = match (item.tweak, setting) {
            (TweakId::StartupApp, Setting::StartupApproval { name, .. }) => {
                item.subject.as_deref() == Some(name.as_str())
            }
            (TweakId::StartupApp, _) => false,
            _ => catalog.contains(&setting.without_scheme()),
        };
        if !belongs {
            return Err(format!("{setting:?} is not part of {:?}", item.tweak));
        }
        if setting.applier() != applier {
            return Err(format!("{setting:?} is not written by {applier:?}"));
        }
        if setting.is_forbidden() {
            return Err(format!("{setting:?} is never changed"));
        }
        if !setting.fits(&change.to) {
            return Err(format!("{:?} does not fit {setting:?}", change.to));
        }
        if item.undo_of.is_none()
            && item.tweak == TweakId::StartupApp
            && !matches!(&change.to, Value::Bytes(b) if b[0] == 3)
        {
            return Err("a startup entry is only ever disabled".into());
        }
    }
    Ok(())
}

/// Reads and writes Windows settings; the helper and the watcher each bring their own.
pub trait SettingStore {
    fn read(&mut self, setting: &Setting) -> Option<Value>;
    /// The Win32 error when the write failed.
    fn write(&mut self, setting: &Setting, value: &Value) -> Result<(), u32>;
}

/// Whether any setting an undo would put back now holds something other than what was written
/// or what it held before; a setting that cannot be read is left to the undo itself to refuse.
pub fn changed_outside(written: &[SettingChange], store: &mut impl SettingStore) -> bool {
    written.iter().any(|change| {
        store
            .read(&change.setting)
            .is_some_and(|now| !same_state(&now, &change.to) && !same_state(&now, &change.from))
    })
}

/// Whether two readings of a setting are the same state; a service's running flag is left out,
/// since it is still moving right after a start or a stop.
fn same_state(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (
            Value::Service { start, delayed, .. },
            Value::Service {
                start: other_start,
                delayed: other_delayed,
                ..
            },
        ) => start == other_start && delayed == other_delayed,
        _ => a == b,
    }
}

/// The setting a change acts on: a plugged-in power value being applied is pinned to the scheme
/// active now, so its undo goes back to that scheme; every other setting is as the change names it.
fn target_of(
    change: &SettingChange,
    undoing: bool,
    store: &mut impl SettingStore,
) -> Option<Setting> {
    match &change.setting {
        Setting::PowerAc {
            subgroup, setting, ..
        } if !undoing => match store.read(&Setting::ActiveScheme) {
            Some(Value::Scheme(scheme)) => Some(Setting::PowerAc {
                subgroup: *subgroup,
                setting: *setting,
                scheme: Some(scheme),
            }),
            _ => None,
        },
        other => Some(other.clone()),
    }
}

/// What the helper recorded before its first change: for every item it will carry out, each
/// setting with the value read and the value it is to write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakIntent {
    pub ts_ms: i64,
    pub items: Vec<IntentItem>,
}

/// One item of a `TweakIntent`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentItem {
    pub tweak: TweakId,
    pub subject: Option<String>,
    pub undo_of: Option<i64>,
    pub settings: Vec<IntentSetting>,
}

/// One setting of an `IntentItem`, as the change will act on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentSetting {
    pub setting: Setting,
    /// What the setting held when the intent was recorded; `None` when it could not be read.
    pub before: Option<Value>,
    pub to: Value,
}

/// Reads every setting the checked `items` will write, to record before the first write.
pub fn intent_of(items: &[&PlanItem], store: &mut impl SettingStore, ts_ms: i64) -> TweakIntent {
    let items = items
        .iter()
        .map(|item| IntentItem {
            tweak: item.tweak,
            subject: item.subject.clone(),
            undo_of: item.undo_of,
            settings: item
                .changes
                .iter()
                .map(|change| {
                    let target = target_of(change, item.undo_of.is_some(), store);
                    IntentSetting {
                        before: target.as_ref().and_then(|setting| store.read(setting)),
                        setting: target.unwrap_or_else(|| change.setting.clone()),
                        to: change.to.clone(),
                    }
                })
                .collect(),
        })
        .collect();
    TweakIntent { ts_ms, items }
}

/// What an intent with no result left in Windows: each setting is found written, found as it was
/// or found otherwise, and recorded as such; the restore point is not known.
pub fn settle_interrupted(intent: &TweakIntent, store: &mut impl SettingStore) -> TweakResults {
    let changes = intent
        .items
        .iter()
        .map(|item| TweakChange {
            tweak: item.tweak,
            subject: item.subject.clone(),
            undo_of: item.undo_of,
            results: item
                .settings
                .iter()
                .map(|planned| settle_setting(planned, store))
                .collect(),
            restore_point: Some(RestorePoint::Unconfirmed),
        })
        .collect();
    TweakResults {
        ts_ms: intent.ts_ms,
        changes,
    }
}

fn settle_setting(planned: &IntentSetting, store: &mut impl SettingStore) -> SettingResult {
    let now = store.read(&planned.setting);
    let held = |value: &Value| now.as_ref().is_some_and(|now| same_state(now, value));
    let error = match &planned.before {
        _ if now.is_none() => ERROR_INTERRUPTED_UNKNOWN,
        Some(before) if held(before) => ERROR_INTERRUPTED_UNCHANGED,
        Some(_) if held(&planned.to) => ERROR_INTERRUPTED_WRITTEN,
        _ => ERROR_INTERRUPTED_UNKNOWN,
    };
    SettingResult {
        change: SettingChange {
            setting: planned.setting.clone(),
            from: planned.before.clone().unwrap_or(Value::Absent),
            to: planned.to.clone(),
        },
        error: Some(error),
    }
}

/// Carries out one checked item: each setting is read again, written only when the tweak still
/// leads to the value asked for, and recorded with what it really held. Stops at the first failure.
pub fn carry_out(item: &PlanItem, store: &mut impl SettingStore) -> TweakChange {
    let mut results = Vec::new();
    for change in &item.changes {
        let target = target_of(change, item.undo_of.is_some(), store);
        let current = target.as_ref().and_then(|setting| store.read(setting));
        let (Some(target), Some(current)) = (target, current) else {
            results.push(SettingResult {
                change: change.clone(),
                error: Some(ERROR_UNREADABLE),
            });
            break;
        };
        let leads_there = if item.undo_of.is_some() {
            same_state(&current, &change.from) || same_state(&current, &change.to)
        } else {
            current == change.to
                || match (item.tweak, &change.to) {
                    (TweakId::StartupApp, Value::Bytes(b)) => b[0] == 3,
                    _ => apply_value(item.tweak, &target, &current).as_ref() == Some(&change.to),
                }
        };
        let error = if !leads_there {
            Some(ERROR_CHANGED_SINCE)
        } else if current == change.to {
            None
        } else {
            match store.write(&target, &change.to) {
                Err(code) => Some(code),
                Ok(()) => store
                    .read(&target)
                    .is_none_or(|now| !same_state(&now, &change.to))
                    .then_some(ERROR_MAY_HAVE_CHANGED),
            }
        };
        results.push(SettingResult {
            change: SettingChange {
                setting: target,
                from: current,
                to: change.to.clone(),
            },
            error,
        });
        if error.is_some() {
            break;
        }
    }
    TweakChange {
        tweak: item.tweak,
        subject: item.subject.clone(),
        undo_of: item.undo_of,
        results,
        restore_point: None,
    }
}
