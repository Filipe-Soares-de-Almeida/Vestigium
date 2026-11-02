use serde::{Deserialize, Serialize};

/// ATA attributes whose raw count above zero is a reason to look at the disk.
pub const WATCHED_ATTRIBUTES: [u8; 5] = [5, 187, 188, 197, 198];
/// NVMe "percentage used" from which the disk counts as worn.
pub const NVME_WORN_PCT: u8 = 90;
const ENTRIES: usize = 30;
const ENTRY_LEN: usize = 12;

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

/// The attributes of a 512-byte SMART READ DATA block: 30 entries of 12 bytes from byte 2.
pub fn smart_attributes(data: &[u8]) -> Vec<SmartAttribute> {
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
