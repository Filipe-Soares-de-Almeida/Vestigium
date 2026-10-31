use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{ALL_PROCESSOR_GROUPS, GetActiveProcessorCount};

use crate::CollectError;

/// System memory, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryInfo {
    pub avail_bytes: u64,
    pub commit_bytes: u64,
    pub commit_limit_bytes: u64,
}

/// Reads physical memory and commit charge; the commit limit is the "total page file" of the API.
pub fn memory_info() -> Result<MemoryInfo, CollectError> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: fills `status`, whose length field is set.
    unsafe { GlobalMemoryStatusEx(&mut status)? };
    Ok(MemoryInfo {
        avail_bytes: status.ullAvailPhys,
        commit_bytes: status
            .ullTotalPageFile
            .saturating_sub(status.ullAvailPageFile),
        commit_limit_bytes: status.ullTotalPageFile,
    })
}

/// Logical processors across all processor groups.
pub fn logical_cpus() -> u32 {
    // SAFETY: a plain query with no pointers.
    unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) }
}
