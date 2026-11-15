use serde::{Deserialize, Serialize};

use crate::shutdown::days_from_civil;
use crate::xml::data_named;

/// Layout of the `BootFacts` BLOB the store keeps.
pub const BOOT_VERSION: u8 = 1;
/// Sign-in later than this after the kernel started is taken as the person at the sign-in screen.
pub const PROBABLE_WAIT_MS: u32 = 30_000;
/// Events up to this long before a boot's first event still belong to it (same-instant logging).
const LEAD_MS: i64 = 5_000;

/// The provider an event of a boot came from; the collector reads each from its own channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootSource {
    KernelGeneral,
    KernelBoot,
    Winlogon,
    Tpm,
    ServiceControl,
    UserProfile,
    ShellCore,
    EventTracing,
}

/// A startup app Explorer ran from a Run key, from Shell-Core 9707 and 9708.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupRun {
    pub command: String,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
}

/// What the event logs say about one boot, in Unix epoch milliseconds; any field may be missing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BootFacts {
    /// When the kernel started (Kernel-General 12), or the boot's first event on a resume.
    pub kernel_start_ms: i64,
    /// Kernel-Boot 27: 0 full boot, 1 hybrid boot (Fast Startup), 2 resume from hibernation.
    pub boot_type: Option<u8>,
    /// Kernel-Boot 30: firmware time from reset to the hand-over to Windows.
    pub firmware_ms: Option<u32>,
    pub signin_ms: Option<i64>,
    pub profile_start_ms: Option<i64>,
    pub profile_end_ms: Option<i64>,
    pub tpm_check_ms: Option<i64>,
    pub tpm_ready_ms: Option<i64>,
    /// Shell-Core 62171 for AllLogonTasks: the desktop's logon tasks are done.
    pub desktop_ms: Option<i64>,
    pub apps: Vec<StartupRun>,
    /// Windows' own boot logger hit its size limit, so Windows did not measure this boot.
    pub logger_full: bool,
    /// Boot or system-start drivers that did not load (Service Control Manager 7026).
    pub failed_drivers: Vec<String>,
}

/// What the signed-in user's Temp folder holds; the profile service walks all of it at sign-in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempSize {
    pub ts_ms: i64,
    pub folders: u32,
    pub files: u32,
    /// The count stopped at its limit, so the folder holds at least this much.
    pub capped: bool,
}

/// The phases of one boot, in milliseconds; `None` where a source is missing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BootPhases {
    pub firmware_ms: Option<u32>,
    /// Kernel start to sign-in; above `PROBABLE_WAIT_MS` it includes the person at the sign-in screen.
    pub to_signin_ms: Option<u32>,
    pub probable_wait: bool,
    pub profile_ms: Option<u32>,
    /// Part of the profile step spent before Windows declared the TPM ready.
    pub tpm_wait_ms: Option<u32>,
    pub desktop_ms: Option<u32>,
    pub to_desktop_ms: Option<u32>,
    /// Firmware plus kernel start to desktop: the time from the power button.
    pub from_power_ms: Option<u32>,
}

/// The milliseconds from `from` to `to`, when both exist and `to` is not earlier.
fn span(from: Option<i64>, to: Option<i64>) -> Option<u32> {
    let ms = to? - from?;
    u32::try_from(ms).ok()
}

impl BootFacts {
    pub fn phases(&self) -> BootPhases {
        let start = Some(self.kernel_start_ms);
        let to_signin_ms = span(start, self.signin_ms);
        let tpm_wait_ms = match (
            self.profile_start_ms,
            self.tpm_ready_ms,
            self.profile_end_ms,
        ) {
            (Some(from), Some(ready), Some(end)) => {
                span(Some(from), Some(ready.min(end)).filter(|r| *r >= from)).or(Some(0))
            }
            _ => None,
        };
        let to_desktop_ms = span(start, self.desktop_ms);
        BootPhases {
            firmware_ms: self.firmware_ms,
            to_signin_ms,
            probable_wait: to_signin_ms.is_some_and(|ms| ms > PROBABLE_WAIT_MS),
            profile_ms: span(self.profile_start_ms, self.profile_end_ms),
            tpm_wait_ms,
            desktop_ms: span(self.profile_end_ms, self.desktop_ms),
            to_desktop_ms,
            from_power_ms: self
                .firmware_ms
                .zip(to_desktop_ms)
                .map(|(fw, desk)| fw.saturating_add(desk)),
        }
    }
}

/// One event as the collector read it: its provider, id, time and `<EventData>` XML.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    source: BootSource,
    id: u16,
    ts_ms: i64,
    text: Option<String>,
    number: Option<i64>,
}

/// The boot events of a span, folded into one `BootFacts` per boot.
#[derive(Clone, Debug, Default)]
pub struct BootLog {
    seen: Vec<Seen>,
    /// Events that could not be read, and so are missing from the boots.
    pub skipped: u32,
}

impl BootLog {
    /// Keeps one event; returns false for an id or data it does not use.
    pub fn add(&mut self, source: BootSource, id: u16, ts_ms: i64, data_xml: &str) -> bool {
        let named = |name| data_named(data_xml, name);
        let (text, number) = match (source, id) {
            (BootSource::KernelGeneral, 12) => (None, named("StartTime").and_then(iso_ms)),
            (BootSource::KernelBoot, 27) => (None, named("BootType").and_then(|v| v.parse().ok())),
            (BootSource::KernelBoot, 30) => (
                None,
                named("ExitBootServicesExit").and_then(|v| v.parse().ok()),
            ),
            (BootSource::Winlogon, 7001)
            | (BootSource::Tpm, 18 | 1025)
            | (BootSource::UserProfile, 1 | 2) => (None, None),
            (BootSource::ServiceControl, 7026) => (named("param1").map(str::to_string), None),
            (BootSource::ShellCore, 62171) => match named("TaskName") {
                Some("AllLogonTasks") => (None, None),
                _ => return false,
            },
            (BootSource::ShellCore, 9707 | 9708) => (named("Command").map(str::to_string), None),
            (BootSource::EventTracing, 4) => match named("SessionName") {
                Some("PerfDiag Logger") => (None, None),
                _ => return false,
            },
            _ => return false,
        };
        self.seen.push(Seen {
            source,
            id,
            ts_ms,
            text,
            number,
        });
        true
    }

    /// One `BootFacts` per boot, oldest first. A boot starts at Kernel-Boot 27, which Windows logs
    /// on full boots, hybrid boots and resumes alike; each event belongs to the boot before it.
    pub fn boots(mut self) -> Vec<BootFacts> {
        self.seen.sort_by_key(|s| s.ts_ms);
        let anchors: Vec<i64> = self
            .seen
            .iter()
            .filter(|s| s.source == BootSource::KernelBoot && s.id == 27)
            .map(|s| s.ts_ms)
            .collect();
        anchors
            .iter()
            .enumerate()
            .map(|(i, &at)| {
                let end = anchors.get(i + 1).map_or(i64::MAX, |next| next - LEAD_MS);
                let window: Vec<&Seen> = self
                    .seen
                    .iter()
                    .filter(|s| s.ts_ms >= at - LEAD_MS && s.ts_ms < end)
                    .collect();
                fold(at, &window)
            })
            .collect()
    }
}

/// The facts of one boot from the events logged between its start and the next boot.
fn fold(at: i64, window: &[&Seen]) -> BootFacts {
    let first =
        |source: BootSource, id: u16| window.iter().find(|s| s.source == source && s.id == id);
    let first_after = |source: BootSource, id: u16, after: i64| {
        window
            .iter()
            .find(|s| s.source == source && s.id == id && s.ts_ms >= after)
            .map(|s| s.ts_ms)
    };
    let kernel = first(BootSource::KernelGeneral, 12);
    let kernel_start_ms = kernel.and_then(|s| s.number).unwrap_or(at);
    let signin_ms = first(BootSource::Winlogon, 7001).map(|s| s.ts_ms);
    let profile_start_ms = first(BootSource::UserProfile, 1).map(|s| s.ts_ms);
    let profile_end_ms =
        profile_start_ms.and_then(|from| first_after(BootSource::UserProfile, 2, from));
    let desktop_ms =
        profile_end_ms.and_then(|from| first_after(BootSource::ShellCore, 62171, from));
    let mut apps: Vec<StartupRun> = Vec::new();
    for s in window.iter().filter(|s| s.source == BootSource::ShellCore) {
        let Some(command) = s.text.clone() else {
            continue;
        };
        match s.id {
            9707 => apps.push(StartupRun {
                command,
                start_ms: s.ts_ms,
                end_ms: None,
            }),
            9708 => {
                if let Some(run) = apps
                    .iter_mut()
                    .find(|r| r.command == command && r.end_ms.is_none())
                {
                    run.end_ms = Some(s.ts_ms);
                }
            }
            _ => {}
        }
    }
    BootFacts {
        kernel_start_ms,
        boot_type: first(BootSource::KernelBoot, 27).and_then(|s| u8::try_from(s.number?).ok()),
        firmware_ms: first(BootSource::KernelBoot, 30).and_then(|s| u32::try_from(s.number?).ok()),
        signin_ms,
        profile_start_ms,
        profile_end_ms,
        tpm_check_ms: first(BootSource::Tpm, 18).map(|s| s.ts_ms),
        tpm_ready_ms: first(BootSource::Tpm, 1025).map(|s| s.ts_ms),
        desktop_ms,
        apps,
        logger_full: first(BootSource::EventTracing, 4).is_some(),
        failed_drivers: window
            .iter()
            .filter(|s| s.source == BootSource::ServiceControl)
            .filter_map(|s| s.text.as_deref())
            .flat_map(str::lines)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect(),
    }
}

/// Unix epoch milliseconds of an ISO 8601 UTC time as Windows writes it ("2026-11-15T18:36:08.5000000Z").
fn iso_ms(text: &str) -> Option<i64> {
    let text = text.strip_suffix('Z')?;
    let (date, time) = text.split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let (year, month, day) = (d.next()??, d.next()??, d.next()??);
    let (hms, frac) = time.split_once('.').unwrap_or((time, "0"));
    let mut t = hms.split(':').map(|p| p.parse::<i64>().ok());
    let (hour, minute, second) = (t.next()??, t.next()??, t.next()??);
    let ms: i64 = format!("{frac:0<3}").get(..3)?.parse().ok()?;
    let valid = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second < 61;
    valid.then(|| {
        ((days_from_civil(year, month, day) * 24 + hour) * 60 + minute) * 60_000
            + second * 1000
            + ms
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_times_read_to_the_millisecond() {
        assert_eq!(iso_ms("1970-01-01T00:00:01.5000000Z"), Some(1500));
        assert_eq!(
            iso_ms("2026-11-15T18:36:08.5000000Z"),
            Some(1_794_767_768_500)
        );
        assert_eq!(iso_ms("2026-11-15T18:36:08Z"), Some(1_794_767_768_000));
        assert_eq!(iso_ms("not a time"), None);
    }
}
