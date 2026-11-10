use serde::{Deserialize, Serialize};

/// ATA attributes whose raw count above zero is a reason to look at the disk.
pub const WATCHED_ATTRIBUTES: [u8; 5] = [5, 187, 188, 197, 198];
/// Seagate packs three 16-bit counters in attribute 188; its first is the command timeouts.
const COMMAND_TIMEOUT_ID: u8 = 188;
/// NVMe "percentage used" from which the disk counts as worn.
pub const NVME_WORN_PCT: u8 = 90;
const ENTRIES: usize = 30;
const ENTRY_LEN: usize = 12;
const BLOCK_LEN: usize = 512;

/// One attribute of an ATA SMART READ DATA block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartAttribute {
    pub id: u8,
    /// Normalized value; the disk compares it with its own threshold.
    pub current: u8,
    pub worst: u8,
    /// Six-byte raw value, vendor-defined; counts for the watched attributes.
    pub raw: u64,
}

/// The NVMe SMART / Health Information log fields the findings read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvmeHealth {
    /// Bit 0 spare below threshold, 1 temperature, 2 reliability degraded, 3 read-only, 4 backup failed.
    pub critical_warning: u8,
    pub temperature_c: i16,
    pub available_spare_pct: u8,
    pub spare_threshold_pct: u8,
    pub percentage_used: u8,
    pub media_errors: u64,
}

/// Everything of the NVMe health log the Tests screen shows; the findings read `NvmeHealth`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvmeDetail {
    pub health: NvmeHealth,
    pub data_read_bytes: u64,
    pub data_written_bytes: u64,
    pub power_cycles: u64,
    pub power_on_hours: u64,
    pub unsafe_shutdowns: u64,
    pub error_log_entries: u64,
}

/// The NVMe log counts data in units of 1000 sectors of 512 bytes.
const NVME_DATA_UNIT: u64 = 512_000;

/// The low 64 bits of the 128-bit counter at `at` in an NVMe health log.
fn counter(log: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(log.get(at..at + 8)?.try_into().ok()?))
}

/// The detail of an NVMe health log; `None` when shorter than the fields it holds.
pub fn nvme_detail(log: &[u8]) -> Option<NvmeDetail> {
    Some(NvmeDetail {
        health: nvme_health(log)?,
        data_read_bytes: counter(log, 32)?.saturating_mul(NVME_DATA_UNIT),
        data_written_bytes: counter(log, 48)?.saturating_mul(NVME_DATA_UNIT),
        power_cycles: counter(log, 112)?,
        power_on_hours: counter(log, 128)?,
        unsafe_shutdowns: counter(log, 144)?,
        error_log_entries: counter(log, 176)?,
    })
}

/// The count a watched attribute stands for: the low 16 bits of 188 (assumed Seagate layout), else the raw value.
pub fn watched_count(attribute: &SmartAttribute) -> u32 {
    if attribute.id == COMMAND_TIMEOUT_ID {
        (attribute.raw & 0xFFFF) as u32
    } else {
        u32::try_from(attribute.raw).unwrap_or(u32::MAX)
    }
}

/// The attributes of a 512-byte SMART READ DATA block: 30 entries of 12 bytes from byte 2.
/// A block whose bytes do not sum to zero modulo 256 yields none.
pub fn smart_attributes(data: &[u8]) -> Vec<SmartAttribute> {
    let sum = data
        .iter()
        .take(BLOCK_LEN)
        .fold(0u8, |a, b| a.wrapping_add(*b));
    if data.len() < BLOCK_LEN || sum != 0 {
        return Vec::new();
    }
    (0..ENTRIES)
        .filter_map(|i| data.get(2 + i * ENTRY_LEN..2 + (i + 1) * ENTRY_LEN))
        .filter(|entry| entry[0] != 0)
        .map(|entry| {
            let mut raw = [0u8; 8];
            raw[..6].copy_from_slice(&entry[5..11]);
            SmartAttribute {
                id: entry[0],
                current: entry[3],
                worst: entry[4],
                raw: u64::from_le_bytes(raw),
            }
        })
        .collect()
}

/// The health log (log page 2) of an NVMe disk; `None` when shorter than the fields it holds.
pub fn nvme_health(log: &[u8]) -> Option<NvmeHealth> {
    let kelvin = u16::from_le_bytes(log.get(1..3)?.try_into().ok()?);
    // Media and data integrity errors is a 128-bit counter at byte 160; the low half is plenty.
    let media_errors = u64::from_le_bytes(log.get(160..168)?.try_into().ok()?);
    Some(NvmeHealth {
        critical_warning: log[0],
        temperature_c: (i32::from(kelvin) - 273).clamp(i16::MIN.into(), i16::MAX.into()) as i16,
        available_spare_pct: log[3],
        spare_threshold_pct: log[4],
        percentage_used: log[5],
        media_errors,
    })
}
