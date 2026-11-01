use std::fmt::Write as _;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// The Windows logs the curated events come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    System,
    Application,
}

impl Channel {
    /// The log's name as the Event Log API spells it.
    pub fn path(self) -> &'static str {
        match self {
            Channel::System => "System",
            Channel::Application => "Application",
        }
    }

    pub fn from_path(path: &str) -> Option<Channel> {
        [Channel::System, Channel::Application]
            .into_iter()
            .find(|channel| channel.path().eq_ignore_ascii_case(path))
    }
}

/// What a curated event says happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventMeaning {
    StorageReset,
    DiskRetry,
    DiskBadBlock,
    PagingError,
    DisplayReset,
    HardwareError,
    PowerLoss,
    UnexpectedShutdown,
    BugCheck,
    CpuFirmwareLimit,
    LowVirtualMemory,
    AppHang,
    AppCrash,
    ServiceCrash,
}

/// One kind of event worth keeping: always a provider and an id, since ids repeat across providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CuratedEvent {
    pub channel: Channel,
    pub providers: &'static [&'static str],
    pub ids: &'static [u16],
    pub meaning: EventMeaning,
}

/// Storage drivers that report a device reset as event 129; the XPath of the Event Log cannot match a prefix.
const STORAGE_DRIVERS: &[&str] = &[
    "storahci",
    "stornvme",
    "iaStorA",
    "iaStorAC",
    "iaStorAVC",
    "iaStorV",
    "iaStorVD",
    "secnvme",
];

/// The events the watcher keeps.
pub const CURATED: &[CuratedEvent] = &[
    curated(
        Channel::System,
        STORAGE_DRIVERS,
        &[129],
        EventMeaning::StorageReset,
    ),
    curated(Channel::System, &["disk"], &[153], EventMeaning::DiskRetry),
    curated(Channel::System, &["disk"], &[7], EventMeaning::DiskBadBlock),
    curated(Channel::System, &["disk"], &[51], EventMeaning::PagingError),
    curated(
        Channel::System,
        &["Display"],
        &[4101],
        EventMeaning::DisplayReset,
    ),
    curated(
        Channel::System,
        &["Microsoft-Windows-WHEA-Logger"],
        &[17, 18, 19, 47],
        EventMeaning::HardwareError,
    ),
    curated(
        Channel::System,
        &["Microsoft-Windows-Kernel-Power"],
        &[41],
        EventMeaning::PowerLoss,
    ),
    curated(
        Channel::System,
        &["EventLog"],
        &[6008],
        EventMeaning::UnexpectedShutdown,
    ),
    curated(
        Channel::System,
        &["Microsoft-Windows-WER-SystemErrorReporting"],
        &[1001],
        EventMeaning::BugCheck,
    ),
    curated(
        Channel::System,
        &["Microsoft-Windows-Kernel-Processor-Power"],
        &[37],
        EventMeaning::CpuFirmwareLimit,
    ),
    curated(
        Channel::System,
        &["Microsoft-Windows-Resource-Exhaustion-Detector"],
        &[2004],
        EventMeaning::LowVirtualMemory,
    ),
    curated(
        Channel::Application,
        &["Application Hang"],
        &[1002],
        EventMeaning::AppHang,
    ),
    curated(
        Channel::Application,
        &["Application Error"],
        &[1000],
        EventMeaning::AppCrash,
    ),
    curated(
        Channel::System,
        &["Service Control Manager"],
        &[7031, 7034],
        EventMeaning::ServiceCrash,
    ),
];

const fn curated(
    channel: Channel,
    providers: &'static [&'static str],
    ids: &'static [u16],
    meaning: EventMeaning,
) -> CuratedEvent {
    CuratedEvent {
        channel,
        providers,
        ids,
        meaning,
    }
}

/// What a raw event means, when it is curated; providers compare without case.
pub fn classify(channel: Channel, provider: &str, id: u16) -> Option<EventMeaning> {
    CURATED
        .iter()
        .find(|entry| {
            entry.channel == channel
                && entry.ids.contains(&id)
                && entry
                    .providers
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(provider))
        })
        .map(|entry| entry.meaning)
}

/// A structured query with one `<Select>` per entry; `backfill` limits it to that much of the past.
pub fn query_xml_for(entries: &[CuratedEvent], backfill: Option<Duration>) -> String {
    let mut xml = String::from("<QueryList><Query Id=\"0\">");
    for entry in entries {
        let providers = entry
            .providers
            .iter()
            .map(|p| format!("@Name='{}'", escape(p)))
            .collect::<Vec<_>>()
            .join(" or ");
        let ids = entry
            .ids
            .iter()
            .map(|id| format!("EventID={id}"))
            .collect::<Vec<_>>()
            .join(" or ");
        let window = backfill.map_or_else(String::new, |span| {
            format!(
                " and TimeCreated[timediff(@SystemTime) &lt;= {}]",
                span.as_millis()
            )
        });
        let _ = write!(
            xml,
            "<Select Path=\"{}\">*[System[(Provider[{providers}]) and ({ids}){window}]]</Select>",
            entry.channel.path()
        );
    }
    xml.push_str("</Query></QueryList>");
    xml
}

/// The structured query for every curated event.
pub fn query_xml(backfill: Option<Duration>) -> String {
    query_xml_for(CURATED, backfill)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// One curated event as the watcher keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WinEvent {
    /// When Windows logged it, Unix epoch milliseconds.
    pub ts_ms: i64,
    pub channel: Channel,
    pub provider: String,
    pub id: u16,
    /// Position in its log, unique per channel.
    pub record_id: u64,
    pub level: u8,
    pub meaning: EventMeaning,
    /// The event's own data, as XML, cut to at most 4 KiB.
    pub data_xml: String,
}
