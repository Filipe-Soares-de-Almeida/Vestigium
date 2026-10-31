use std::ffi::c_void;
use std::ptr;

use watchmypc_core::ProcessCounters;
use windows::Wdk::System::SystemInformation::{NtQuerySystemInformation, SystemProcessInformation};
use windows::Win32::Foundation::STATUS_INFO_LENGTH_MISMATCH;

use crate::CollectError;

/// x64 layout of SYSTEM_PROCESS_INFORMATION; the crate only has the documented, reserved-field one.
#[repr(C)]
#[derive(Clone, Copy)]
struct ProcessEntry {
    next_entry_offset: u32,
    number_of_threads: u32,
    working_set_private_size: i64,
    hard_fault_count: u32,
    number_of_threads_high_watermark: u32,
    cycle_time: u64,
    create_time: i64,
    user_time: i64,
    kernel_time: i64,
    image_name_length: u16,
    image_name_maximum_length: u16,
    image_name_buffer: *const u16,
    base_priority: i32,
    unique_process_id: usize,
    inherited_from_unique_process_id: usize,
    handle_count: u32,
    session_id: u32,
    unique_process_key: usize,
    peak_virtual_size: usize,
    virtual_size: usize,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_page_count: usize,
    read_operation_count: i64,
    write_operation_count: i64,
    other_operation_count: i64,
    read_transfer_count: i64,
    write_transfer_count: i64,
    other_transfer_count: i64,
}

/// x64 layout of SYSTEM_THREAD_INFORMATION; an array of them follows each process entry.
#[repr(C)]
#[derive(Clone, Copy)]
struct ThreadEntry {
    kernel_time: i64,
    user_time: i64,
    create_time: i64,
    wait_time: u32,
    start_address: usize,
    client_id: [usize; 2],
    priority: i32,
    base_priority: i32,
    context_switches: u32,
    thread_state: u32,
    wait_reason: u32,
}

const _: () = assert!(size_of::<ProcessEntry>() == 0x100);
const _: () = assert!(size_of::<ThreadEntry>() == 0x50);

/// Reads every process with one NtQuerySystemInformation call, reusing its buffer.
#[derive(Debug)]
pub struct ProcessSnapshotter {
    buf: Vec<u64>,
}

impl ProcessSnapshotter {
    pub fn with_capacity(bytes: usize) -> ProcessSnapshotter {
        ProcessSnapshotter {
            buf: vec![0; bytes.div_ceil(8).max(1)],
        }
    }

    pub fn snapshot(&mut self) -> Result<Vec<ProcessCounters>, CollectError> {
        let filled = loop {
            let capacity = u32::try_from(self.buf.len() * 8).unwrap_or(u32::MAX);
            let mut needed = 0u32;
            // SAFETY: the buffer has `capacity` writable bytes, aligned to 8.
            let status = unsafe {
                NtQuerySystemInformation(
                    SystemProcessInformation,
                    self.buf.as_mut_ptr().cast::<c_void>(),
                    capacity,
                    &mut needed,
                )
            };
            if status == STATUS_INFO_LENGTH_MISMATCH {
                let bytes = needed as usize + 64 * 1024;
                self.buf.resize(bytes.div_ceil(8), 0);
                continue;
            }
            status.ok()?;
            break (needed as usize).min(self.buf.len() * 8);
        };
        // SAFETY: the kernel wrote `filled` bytes of process entries at the start of the buffer.
        Ok(unsafe { parse(self.buf.as_ptr().cast::<u8>(), filled) })
    }
}

/// Walks the entry chain, never reading past `len` bytes.
///
/// # Safety
/// `base` must point at `len` bytes written by NtQuerySystemInformation(SystemProcessInformation).
unsafe fn parse(base: *const u8, len: usize) -> Vec<ProcessCounters> {
    let mut processes = Vec::with_capacity(512);
    let mut offset = 0usize;
    while offset + size_of::<ProcessEntry>() <= len {
        // SAFETY: the entry lies inside the written bytes, checked above.
        let entry = unsafe { ptr::read_unaligned(base.add(offset).cast::<ProcessEntry>()) };
        let threads_at = offset + size_of::<ProcessEntry>();
        let threads = entry.number_of_threads as usize;
        let mut context_switches = 0u64;
        if threads_at + threads * size_of::<ThreadEntry>() <= len {
            for i in 0..threads {
                // SAFETY: each thread entry lies inside the written bytes, checked above.
                let thread = unsafe {
                    ptr::read_unaligned(
                        base.add(threads_at + i * size_of::<ThreadEntry>())
                            .cast::<ThreadEntry>(),
                    )
                };
                context_switches += u64::from(thread.context_switches);
            }
        }
        processes.push(ProcessCounters {
            pid: entry.unique_process_id as u32,
            parent_pid: entry.inherited_from_unique_process_id as u32,
            // SAFETY: a non-null name buffer points at `length` bytes of UTF-16 inside the snapshot.
            name: unsafe { image_name(&entry) },
            create_time: entry.create_time,
            cpu_time_100ns: (entry.user_time.max(0) + entry.kernel_time.max(0)) as u64,
            working_set: entry.working_set_size as u64,
            private_bytes: entry.private_page_count as u64,
            read_bytes: entry.read_transfer_count.max(0) as u64,
            write_bytes: entry.write_transfer_count.max(0) as u64,
            threads: entry.number_of_threads,
            context_switches,
        });
        if entry.next_entry_offset == 0 {
            break;
        }
        offset += entry.next_entry_offset as usize;
    }
    processes
}

/// # Safety
/// The entry must come from a live snapshot buffer.
unsafe fn image_name(entry: &ProcessEntry) -> String {
    if entry.image_name_buffer.is_null() || entry.image_name_length == 0 {
        return if entry.unique_process_id == 0 {
            "Idle".to_string()
        } else {
            String::new()
        };
    }
    let units = usize::from(entry.image_name_length) / 2;
    // SAFETY: guaranteed by the caller; the kernel points the name inside the snapshot.
    let name = unsafe { std::slice::from_raw_parts(entry.image_name_buffer, units) };
    String::from_utf16_lossy(name)
}
