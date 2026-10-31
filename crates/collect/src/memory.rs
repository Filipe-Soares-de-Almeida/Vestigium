use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows::Win32::System::Threading::{ALL_PROCESSOR_GROUPS, GetActiveProcessorCount};

use crate::CollectError;

/// System memory, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryInfo {
    pub avail_bytes: u64,
    pub commit_bytes: u64,
    pub commit_limit_bytes: u64,
}

pub fn memory_info() -> Result<MemoryInfo, CollectError> {
    let mut info = PERFORMANCE_INFORMATION::default();
    // SAFETY: fills `info`, whose size is passed alongside it.
    unsafe { GetPerformanceInfo(&mut info, size_of::<PERFORMANCE_INFORMATION>() as u32)? };
    let page = info.PageSize as u64;
    Ok(MemoryInfo {
        avail_bytes: info.PhysicalAvailable as u64 * page,
        commit_bytes: info.CommitTotal as u64 * page,
        commit_limit_bytes: info.CommitLimit as u64 * page,
    })
}

/// Logical processors across all processor groups.
pub fn logical_cpus() -> u32 {
    // SAFETY: a plain query with no pointers.
    unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) }
}
