use crate::xml::data_named;

use super::{BootDelay, BootReport, BootTime, DelayKind};

/// Words that mark a slow boot step as a security product, which is never offered for removal.
const SECURITY_WORDS: [&str; 5] = [
    "defender",
    "antimalware",
    "antivirus",
    "security",
    "msmpeng",
];

impl BootReport {
    /// Keeps one Diagnostics-Performance event: 100 is a measured boot, 101 to 110 a step that
    /// slowed it. Returns false for any other id or data it cannot read.
    pub fn add(&mut self, id: u16, ts_ms: i64, data_xml: &str) -> bool {
        let number = |name| data_named(data_xml, name).and_then(|v| v.parse::<u32>().ok());
        match id {
            100 => {
                let Some(boot_ms) = number("BootTime") else {
                    return false;
                };
                self.boots.push(BootTime {
                    ts_ms,
                    boot_ms,
                    main_path_ms: number("MainPathBootTime").unwrap_or(0),
                    post_boot_ms: number("BootPostBootTime").unwrap_or(0),
                });
                true
            }
            101..=110 => {
                let name = ["FriendlyName", "Name"]
                    .into_iter()
                    .filter_map(|field| data_named(data_xml, field))
                    .find(|v| !v.is_empty());
                let (Some(name), Some(total_ms)) = (name, number("TotalTime")) else {
                    return false;
                };
                let file = data_named(data_xml, "Name").unwrap_or_default();
                let security = SECURITY_WORDS.iter().any(|word| {
                    name.to_lowercase().contains(word) || file.to_lowercase().contains(word)
                });
                self.delays.push(BootDelay {
                    ts_ms,
                    kind: match id {
                        101 => DelayKind::App,
                        102 => DelayKind::Driver,
                        103 => DelayKind::Service,
                        109 => DelayKind::Device,
                        _ => DelayKind::Phase,
                    },
                    name: name.to_string(),
                    total_ms,
                    degradation_ms: number("DegradationTime").unwrap_or(0),
                    security,
                });
                true
            }
            _ => false,
        }
    }
}
