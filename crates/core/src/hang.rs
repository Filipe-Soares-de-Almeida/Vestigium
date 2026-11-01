use serde::{Deserialize, Serialize};

/// A process with a window Windows reports as not responding; window titles are never kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HungWindow {
    pub pid: u32,
    pub name: String,
}
