use serde::{Deserialize, Serialize};

/// A damaged history file the watcher set aside, and what its new history took over from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryReset {
    /// Name of the file set aside, without its folder.
    pub file: String,
    /// When the history was replaced; Unix epoch milliseconds.
    pub at_ms: i64,
    /// The tweak changes were copied into the new history.
    pub tweaks: bool,
    /// The settings were copied.
    pub settings: bool,
    /// The facts about this PC were copied.
    pub machine: bool,
}

impl HistoryReset {
    /// Everything the person had before was carried over.
    pub fn kept_all(&self) -> bool {
        self.tweaks && self.settings && self.machine
    }
}
