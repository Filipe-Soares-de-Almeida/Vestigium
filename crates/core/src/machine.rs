use serde::{Deserialize, Serialize};

/// Whether a disk has moving parts, as the rules want to know.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Media {
    Ssd,
    Hdd,
    Unknown,
}

/// One physical disk, named as the disk counters name it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiskInfo {
    /// Instance name as Windows reports it, such as "0 C:".
    pub name: String,
    pub media: Media,
}

/// Facts about the machine that change rarely, read when watching starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineInfo {
    pub logical_cpus: u32,
    pub mem_total_bytes: u64,
    pub disks: Vec<DiskInfo>,
}
