//! Health sources read without elevation: page files, fixed volumes, the system disk and the
//! security products Windows knows, with what each costs: `health_sources`.

use std::ffi::c_void;
use std::time::Instant;

use vestigium_collect::{ProcessSnapshotter, memory_info};
use windows::Wdk::System::SystemInformation::{NtQuerySystemInformation, SYSTEM_INFORMATION_CLASS};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, GetDiskFreeSpaceExW,
    GetDriveTypeW, GetLogicalDriveStringsW, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::SecurityCenter::{
    IWSCProductList, WSC_SECURITY_PROVIDER, WSC_SECURITY_PROVIDER_ANTISPYWARE,
    WSC_SECURITY_PROVIDER_ANTIVIRUS, WSC_SECURITY_PROVIDER_FIREWALL, WSCProductList,
};
use windows::Win32::System::SystemInformation::GetSystemWindowsDirectoryW;
use windows::core::HSTRING;

/// Undocumented class that lists page files (SYSTEM_PAGEFILE_INFORMATION entries).
const SYSTEM_PAGE_FILE_INFORMATION: SYSTEM_INFORMATION_CLASS = SYSTEM_INFORMATION_CLASS(18);
const PAGE: u64 = 4096;
const DRIVE_FIXED: u32 = 3;

fn main() {
    page_files();
    volumes();
    system_disk();
    security_products();
}

fn ms_since(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn page_files() {
    let mut buf = vec![0u64; 512];
    let mut needed = 0u32;
    let started = Instant::now();
    // SAFETY: the buffer has the writable length passed, aligned to 8.
    let status = unsafe {
        NtQuerySystemInformation(
            SYSTEM_PAGE_FILE_INFORMATION,
            buf.as_mut_ptr().cast::<c_void>(),
            (buf.len() * 8) as u32,
            &mut needed,
        )
    };
    println!(
        "page files: status {:#x}, {needed} bytes ({:.3} ms)",
        status.0,
        ms_since(started)
    );
    if status.0 != 0 {
        return;
    }
    let bytes: Vec<u8> = buf.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut at = 0usize;
    for _ in 0..16 {
        let field =
            |off: usize| u32::from_le_bytes(bytes[at + off..at + off + 4].try_into().unwrap());
        let (next, total, in_use, peak) = (field(0), field(4), field(8), field(12));
        let name_len = u16::from_le_bytes(bytes[at + 16..at + 18].try_into().unwrap()) as usize;
        let name_ptr =
            u64::from_le_bytes(bytes[at + 24..at + 32].try_into().unwrap()) as *const u16;
        // SAFETY: the kernel points the name into the same buffer, `name_len` bytes long.
        let name =
            unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(name_ptr, name_len / 2)) };
        if total > 0 || name_len > 0 {
            println!(
                "  {name}: size {} MB, in use {} MB, peak {} MB",
                (u64::from(total) * PAGE) >> 20,
                (u64::from(in_use) * PAGE) >> 20,
                (u64::from(peak) * PAGE) >> 20
            );
        }
        if next == 0 {
            break;
        }
        at += next as usize;
    }
    if let Ok(mem) = memory_info() {
        println!(
            "  commit {} MB of limit {} MB ({:.0}%), physical {} MB",
            mem.commit_bytes >> 20,
            mem.commit_limit_bytes >> 20,
            mem.commit_bytes as f64 * 100.0 / mem.commit_limit_bytes.max(1) as f64,
            mem.total_bytes >> 20
        );
    }
}

fn volumes() {
    let mut names = [0u16; 256];
    // SAFETY: the buffer is writable for its whole length.
    let len = unsafe { GetLogicalDriveStringsW(Some(&mut names)) } as usize;
    for root in names[..len.min(names.len())]
        .split(|c| *c == 0)
        .filter(|r| !r.is_empty())
    {
        let root = HSTRING::from_wide(root);
        // SAFETY: `root` is a NUL-terminated root path such as "C:\".
        let kind = unsafe { GetDriveTypeW(&root) };
        if kind != DRIVE_FIXED {
            continue;
        }
        let (mut free, mut total) = (0u64, 0u64);
        let started = Instant::now();
        // SAFETY: both outputs outlive the call.
        let asked = unsafe { GetDiskFreeSpaceExW(&root, Some(&mut free), Some(&mut total), None) };
        println!(
            "volume {root}: ok={} free {} GB of {} GB ({:.1}%) ({:.3} ms)",
            asked.is_ok(),
            free >> 30,
            total >> 30,
            free as f64 * 100.0 / total.max(1) as f64,
            ms_since(started)
        );
    }
}

fn system_disk() {
    let mut dir = [0u16; 260];
    // SAFETY: the buffer is writable for its whole length.
    let len = unsafe { GetSystemWindowsDirectoryW(Some(&mut dir)) } as usize;
    let windows_dir = String::from_utf16_lossy(&dir[..len.min(dir.len())]);
    let letter = windows_dir.get(..2).unwrap_or("C:").to_string();
    let path = HSTRING::from(format!(r"\\.\{letter}"));
    // SAFETY: opens the volume with no access rights, for a query only; closed below.
    let opened = unsafe {
        CreateFileW(
            &path,
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    };
    let volume = match opened {
        Ok(volume) => volume,
        Err(err) => {
            println!("system volume {letter}: open failed {:#010x}", err.code().0);
            return;
        }
    };
    let mut out = [0u8; 256];
    let mut returned = 0u32;
    let started = Instant::now();
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
    match asked {
        Ok(()) => {
            let count = u32::from_le_bytes(out[0..4].try_into().unwrap()) as usize;
            let disks: Vec<u32> = (0..count.min(8))
                .map(|i| u32::from_le_bytes(out[8 + i * 24..12 + i * 24].try_into().unwrap()))
                .collect();
            println!(
                "system volume {letter} ({windows_dir}): on physical drive(s) {disks:?} ({:.3} ms)",
                ms_since(started)
            );
        }
        Err(err) => println!(
            "system volume {letter}: extents failed {:#010x}",
            err.code().0
        ),
    }
}

fn own_memory(snapshotter: &mut ProcessSnapshotter) -> (u64, u64) {
    let pid = std::process::id();
    snapshotter
        .snapshot()
        .ok()
        .and_then(|all| all.into_iter().find(|p| p.pid == pid))
        .map(|p| (p.working_set >> 10, p.private_bytes >> 10))
        .unwrap_or_default()
}

fn security_products() {
    let mut snapshotter = ProcessSnapshotter::with_capacity(1 << 20);
    let before = own_memory(&mut snapshotter);
    // SAFETY: initializes COM for this thread; paired with CoUninitialize below.
    let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    println!("\ncom init: {:#x}", init.0);
    for (provider, name) in [
        (WSC_SECURITY_PROVIDER_ANTIVIRUS, "antivirus"),
        (WSC_SECURITY_PROVIDER_ANTISPYWARE, "antispyware"),
        (WSC_SECURITY_PROVIDER_FIREWALL, "firewall"),
    ] {
        let started = Instant::now();
        match products(provider) {
            Ok(list) => {
                println!(
                    "{name}: {} product(s) ({:.1} ms)",
                    list.len(),
                    ms_since(started)
                );
                for (product, state, signature) in list {
                    println!(
                        "  {product}: state {state} (0 on, 1 off, 2 snoozed, 3 expired), signature {signature}"
                    );
                }
            }
            Err(err) => println!(
                "{name}: {:#010x} ({:.1} ms)",
                err.code().0,
                ms_since(started)
            ),
        }
    }
    let after = own_memory(&mut snapshotter);
    // SAFETY: balances the CoInitializeEx above on the same thread.
    unsafe { CoUninitialize() };
    let released = own_memory(&mut snapshotter);
    println!(
        "own memory (working set, private KB): before {before:?}, after {after:?}, after CoUninitialize {released:?}"
    );
}

fn products(provider: WSC_SECURITY_PROVIDER) -> windows::core::Result<Vec<(String, i32, i32)>> {
    // SAFETY: COM is initialized on this thread; the list is released when dropped.
    let list: IWSCProductList =
        unsafe { CoCreateInstance(&WSCProductList, None, CLSCTX_INPROC_SERVER)? };
    // SAFETY: calls on a live interface pointer.
    unsafe {
        list.Initialize(provider)?;
        let count = list.Count()?;
        let mut out = Vec::new();
        for i in 0..count.max(0) as u32 {
            let item = list.get_Item(i)?;
            out.push((
                item.ProductName()?.to_string(),
                item.ProductState()?.0,
                item.SignatureStatus()?.0,
            ));
        }
        Ok(out)
    }
}
