use std::ffi::c_void;

use vestigium_core::health::{PageFile, VolumeSpace};
use windows::Wdk::System::SystemInformation::{NtQuerySystemInformation, SYSTEM_INFORMATION_CLASS};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, GetDiskFreeSpaceExW,
    GetDriveTypeW, GetLogicalDriveStringsW, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::SystemInformation::GetSystemWindowsDirectoryW;
use windows::core::HSTRING;

/// Undocumented class listing the page files as SYSTEM_PAGEFILE_INFORMATION entries.
const SYSTEM_PAGE_FILE_INFORMATION: SYSTEM_INFORMATION_CLASS = SYSTEM_INFORMATION_CLASS(18);
const PAGE: u64 = 4096;
const DRIVE_FIXED: u32 = 3;
/// Size of one SYSTEM_PAGEFILE_INFORMATION entry on x64, without its name.
const ENTRY_LEN: usize = 32;

/// The drive letter of the Windows folder, such as "C:".
fn windows_letter() -> Option<String> {
    let mut dir = [0u16; 260];
    // SAFETY: the buffer is writable for its whole length.
    let len = unsafe { GetSystemWindowsDirectoryW(Some(&mut dir)) } as usize;
    let dir = String::from_utf16_lossy(dir.get(..len)?);
    (dir.get(1..2) == Some(":")).then(|| dir[..2].to_uppercase())
}

/// Free and total space of every fixed volume with a drive letter.
pub fn volumes() -> Vec<VolumeSpace> {
    let system = windows_letter();
    let mut names = [0u16; 512];
    // SAFETY: the buffer is writable for its whole length.
    let len = unsafe { GetLogicalDriveStringsW(Some(&mut names)) } as usize;
    names[..len.min(names.len())]
        .split(|c| *c == 0)
        .filter(|root| !root.is_empty())
        .filter_map(|root| {
            let root_text = String::from_utf16_lossy(root);
            let wide = HSTRING::from(root_text.as_str());
            // SAFETY: `wide` is a NUL-terminated root path such as "C:\".
            if unsafe { GetDriveTypeW(&wide) } != DRIVE_FIXED {
                return None;
            }
            let (mut free, mut total) = (0u64, 0u64);
            // SAFETY: both outputs outlive the call.
            unsafe { GetDiskFreeSpaceExW(&wide, Some(&mut free), Some(&mut total), None) }.ok()?;
            Some(VolumeSpace {
                system: system
                    .as_deref()
                    .is_some_and(|letter| root_text.to_uppercase().starts_with(letter)),
                root: root_text,
                free_bytes: free,
                total_bytes: total,
            })
        })
        .collect()
}

/// The physical drive that holds the Windows folder; a volume over several drives gives the first.
pub fn system_drive() -> Option<u32> {
    volume_drive(&windows_letter()?)
}

/// The physical drive under the volume of `letter` ("C:" or "C:\"), asked of the volume opened
/// with no access rights; a volume spread over several drives answers with the first.
pub fn volume_drive(letter: &str) -> Option<u32> {
    let path = HSTRING::from(format!(r"\\.\{}", letter.trim_end_matches('\\')));
    // SAFETY: opens the volume for a query only; closed below.
    let volume = unsafe {
        CreateFileW(
            &path,
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
    .ok()?;
    let mut out = [0u8; 256];
    let mut returned = 0u32;
    // SAFETY: the output buffer is live and its length is the one passed.
    let asked = unsafe {
        DeviceIoControl(
            volume,
            IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
            None,
            0,
            Some(out.as_mut_ptr().cast::<c_void>()),
            out.len() as u32,
            Some(&mut returned),
            None,
        )
    };
    // SAFETY: `volume` was opened above and is closed exactly once.
    let _ = unsafe { CloseHandle(volume) };
    asked.ok()?;
    // VOLUME_DISK_EXTENTS: a count, padding, then DISK_EXTENT entries starting with DiskNumber.
    let count = u32::from_le_bytes(out[0..4].try_into().ok()?);
    (count > 0).then(|| u32::from_le_bytes(out[8..12].try_into().unwrap_or_default()))
}

/// Every page file with its size and peak use; an empty list means paging is off.
pub fn page_files() -> Option<Vec<PageFile>> {
    let mut buf = vec![0u64; 512];
    let mut needed = 0u32;
    // SAFETY: the buffer has the writable length passed, aligned to 8.
    unsafe {
        NtQuerySystemInformation(
            SYSTEM_PAGE_FILE_INFORMATION,
            buf.as_mut_ptr().cast::<c_void>(),
            (buf.len() * 8) as u32,
            &mut needed,
        )
    }
    .ok()
    .ok()?;
    let filled = (needed as usize).min(buf.len() * 8);
    // SAFETY: the kernel wrote `filled` bytes of entries at the start of the buffer.
    Some(unsafe { parse(buf.as_ptr().cast::<u8>(), filled) })
}

/// Walks the entry chain, never reading past `len` bytes.
///
/// # Safety
/// `base` must point at `len` bytes written by NtQuerySystemInformation(SystemPageFileInformation).
unsafe fn parse(base: *const u8, len: usize) -> Vec<PageFile> {
    let mut files = Vec::new();
    let mut at = 0usize;
    while at + ENTRY_LEN <= len {
        // SAFETY: the entry lies inside the `len` bytes the caller vouches for.
        let entry = unsafe { std::slice::from_raw_parts(base.add(at), ENTRY_LEN) };
        let field =
            |off: usize| u32::from_le_bytes(entry[off..off + 4].try_into().unwrap_or_default());
        let name_len = usize::from(u16::from_le_bytes([entry[16], entry[17]]));
        let name_ptr =
            u64::from_le_bytes(entry[24..32].try_into().unwrap_or_default()) as *const u8;
        let inside = !name_ptr.is_null()
            && (name_ptr as usize) >= base as usize
            && (name_ptr as usize) + name_len <= base as usize + len;
        let path = if inside {
            // SAFETY: the name lies inside the same buffer, as checked just above.
            let bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            String::from_utf16_lossy(&units)
                .trim_start_matches(r"\??\")
                .to_string()
        } else {
            String::new()
        };
        files.push(PageFile {
            path,
            size_bytes: u64::from(field(4)) * PAGE,
            peak_bytes: u64::from(field(12)) * PAGE,
        });
        match field(0) as usize {
            0 => break,
            next => at += next,
        }
    }
    files
}
