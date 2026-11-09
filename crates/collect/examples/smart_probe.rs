//! Disk health queries per physical drive, opened with no access rights: `smart_probe [--rw]`,
//! or `smart_probe --watch <drive> --minutes <m>` to see whether a query wakes a spun-down disk.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    BusTypeNvme, CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    AtaDataTypeIdentify, AtaDataTypeLogPage, IOCTL_STORAGE_PREDICT_FAILURE,
    IOCTL_STORAGE_QUERY_PROPERTY, NVMeDataTypeLogPage, ProtocolTypeAta, ProtocolTypeNvme,
    READ_ATTRIBUTES, READ_THRESHOLDS, SMART_CMD, SMART_CYL_HI, SMART_CYL_LOW, SMART_RCV_DRIVE_DATA,
    SMART_RCV_DRIVE_DATA_EX, StorageDeviceManagementStatus, StorageDeviceProperty,
    StorageDeviceProtocolSpecificProperty, StorageDeviceTemperatureProperty,
};
use windows::Win32::System::Power::GetDevicePowerState;
use windows::core::{BOOL, HSTRING};

/// SMART attributes worth naming in the output.
const WATCHED: [u8; 9] = [5, 9, 187, 188, 194, 197, 198, 199, 231];

fn main() {
    if let Some(drive) = arg("--watch").and_then(|s| s.parse().ok()) {
        let minutes = arg("--minutes").and_then(|s| s.parse().ok()).unwrap_or(40);
        return watch(drive, minutes);
    }
    let rw = std::env::args().any(|a| a == "--rw");
    println!("access: {}", if rw { "read+write" } else { "none" });
    for index in 0..16u8 {
        let Some(device) = open(index, rw) else {
            continue;
        };
        println!("\ndrive {index}");
        power(device, "before");
        let bus = bus_type(device);
        predict_failure(device);
        smart(device, index, SMART_RCV_DRIVE_DATA, "smart_rcv_drive_data");
        smart(
            device,
            index,
            SMART_RCV_DRIVE_DATA_EX,
            "smart_rcv_drive_data_ex",
        );
        ata_protocol(device);
        if bus == Some(BusTypeNvme.0) {
            nvme_health(device);
        }
        management_status(device);
        temperature(device);
        power(device, "after");
        // SAFETY: `device` was opened by `open` and is closed exactly once.
        let _ = unsafe { CloseHandle(device) };
    }
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

/// Polls the power state every 30 s; the first time the disk reads as off, times one SMART read.
fn watch(drive: u8, minutes: u64) {
    let Some(device) = open(drive, false) else {
        return;
    };
    let started = Instant::now();
    let mut queried = false;
    while started.elapsed() < Duration::from_secs(minutes * 60) {
        let mut on = BOOL(0);
        // SAFETY: `on` outlives the call; the handle is a live device handle.
        let _ = unsafe { GetDevicePowerState(device, &mut on) };
        let at = started.elapsed().as_secs();
        println!("{at:5} s: on={}", on.as_bool());
        if !on.as_bool() && !queried {
            queried = true;
            let (answer, ms) = ioctl(device, IOCTL_STORAGE_PREDICT_FAILURE, &[], 4 + 512);
            println!(
                "{at:5} s: predict_failure ok={} in {ms:.0} ms",
                answer.is_ok()
            );
            power(device, "right after");
        }
        std::thread::sleep(Duration::from_secs(30));
    }
    // SAFETY: `device` was opened above and is closed exactly once.
    let _ = unsafe { CloseHandle(device) };
}

fn open(index: u8, rw: bool) -> Option<HANDLE> {
    let path = HSTRING::from(format!(r"\\.\PhysicalDrive{index}"));
    let access = if rw {
        (GENERIC_READ | GENERIC_WRITE).0
    } else {
        0
    };
    // SAFETY: opens the device for queries only; the caller closes the handle.
    let opened = unsafe {
        CreateFileW(
            &path,
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    };
    match opened {
        Ok(device) => Some(device),
        Err(err) if err.code().0 as u32 == 0x8007_0002 => None,
        Err(err) => {
            println!("\ndrive {index}: open failed {:#010x}", err.code().0);
            None
        }
    }
}

/// Sends one control code and returns the bytes it wrote, timing the call.
fn ioctl(
    device: HANDLE,
    code: u32,
    input: &[u8],
    out_len: usize,
) -> (Result<Vec<u8>, String>, f64) {
    let mut out = vec![0u8; out_len];
    let mut returned = 0u32;
    let started = Instant::now();
    // SAFETY: both buffers are live for the call and their lengths are the ones passed.
    let asked = unsafe {
        DeviceIoControl(
            device,
            code,
            Some(input.as_ptr().cast::<c_void>()),
            input.len() as u32,
            Some(out.as_mut_ptr().cast::<c_void>()),
            out.len() as u32,
            Some(&mut returned),
            None,
        )
    };
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    match asked {
        Ok(()) => {
            out.truncate(returned as usize);
            (Ok(out), ms)
        }
        Err(err) => (Err(format!("{:#010x}", err.code().0)), ms),
    }
}

/// A STORAGE_PROPERTY_QUERY for `property` (standard query) followed by `extra` parameters.
fn property_query(property: i32, extra: &[u8]) -> Vec<u8> {
    let mut query = Vec::with_capacity(12 + extra.len());
    query.extend_from_slice(&property.to_le_bytes());
    query.extend_from_slice(&0i32.to_le_bytes());
    if extra.is_empty() {
        query.extend_from_slice(&[0; 4]);
    } else {
        query.extend_from_slice(extra);
    }
    query
}

/// STORAGE_PROTOCOL_SPECIFIC_DATA asking for `length` bytes right after itself.
fn protocol_data(
    protocol: i32,
    data_type: i32,
    value: u32,
    sub_value: u32,
    length: u32,
) -> Vec<u8> {
    let fields: [u32; 10] = [
        protocol as u32,
        data_type as u32,
        value,
        sub_value,
        40,
        length,
        0,
        0,
        0,
        0,
    ];
    let mut bytes: Vec<u8> = fields.iter().flat_map(|f| f.to_le_bytes()).collect();
    bytes.resize(40 + length as usize, 0);
    bytes
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn power(device: HANDLE, when: &str) {
    let mut on = BOOL(0);
    let started = Instant::now();
    // SAFETY: `on` outlives the call; the handle is a live device handle.
    let asked = unsafe { GetDevicePowerState(device, &mut on) };
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "  power {when}: asked={} on={} ({ms:.2} ms)",
        asked.as_bool(),
        on.as_bool()
    );
}

fn bus_type(device: HANDLE) -> Option<i32> {
    let (answer, ms) = ioctl(
        device,
        IOCTL_STORAGE_QUERY_PROPERTY,
        &property_query(StorageDeviceProperty.0, &[]),
        1024,
    );
    match answer {
        Ok(bytes) => {
            let bus = u32_at(&bytes, 28).map(|b| b as i32);
            println!("  device property: bus {bus:?} ({ms:.2} ms)");
            bus
        }
        Err(code) => {
            println!("  device property: {code} ({ms:.2} ms)");
            None
        }
    }
}

/// SMART attribute entries (id, current, worst, raw) from a 512-byte SMART READ DATA block.
fn attributes(data: &[u8]) -> Vec<(u8, u8, u8, u64)> {
    (0..30)
        .filter_map(|i| {
            let entry = data.get(2 + i * 12..2 + i * 12 + 12)?;
            if entry[0] == 0 {
                return None;
            }
            let mut raw = [0u8; 8];
            raw[..6].copy_from_slice(&entry[5..11]);
            Some((entry[0], entry[3], entry[4], u64::from_le_bytes(raw)))
        })
        .collect()
}

fn print_attributes(data: &[u8]) {
    let all = attributes(data);
    let ids: Vec<u8> = all.iter().map(|a| a.0).collect();
    println!("    {} attributes: {ids:?}", all.len());
    for (id, current, worst, raw) in all.iter().filter(|a| WATCHED.contains(&a.0)) {
        println!("    id {id:3}: current {current:3} worst {worst:3} raw {raw}");
    }
}

fn predict_failure(device: HANDLE) {
    let (answer, ms) = ioctl(device, IOCTL_STORAGE_PREDICT_FAILURE, &[], 4 + 512);
    match answer {
        Ok(bytes) => {
            println!(
                "  predict_failure: ok, {} bytes, predict={:?} ({ms:.2} ms)",
                bytes.len(),
                u32_at(&bytes, 0)
            );
            if bytes.len() >= 4 + 512 {
                print_attributes(&bytes[4..]);
            }
        }
        Err(code) => println!("  predict_failure: {code} ({ms:.2} ms)"),
    }
}

/// SENDCMDINPARAMS (packed) for a SMART read with `feature` on drive `index`.
fn smart_command(index: u8, feature: u32) -> Vec<u8> {
    let mut cmd = Vec::with_capacity(32);
    cmd.extend_from_slice(&512u32.to_le_bytes());
    cmd.extend_from_slice(&[
        feature as u8,
        1,
        1,
        SMART_CYL_LOW as u8,
        SMART_CYL_HI as u8,
        0xA0 | ((index & 1) << 4),
        SMART_CMD as u8,
        0,
    ]);
    cmd.push(index);
    cmd.extend_from_slice(&[0; 3 + 16]);
    cmd
}

fn smart(device: HANDLE, index: u8, code: u32, name: &str) {
    for (feature, what) in [
        (READ_ATTRIBUTES, "attributes"),
        (READ_THRESHOLDS, "thresholds"),
    ] {
        let (answer, ms) = ioctl(device, code, &smart_command(index, feature), 16 + 512);
        match answer {
            Ok(bytes) if bytes.len() >= 16 + 512 => {
                println!("  {name} {what}: ok ({ms:.2} ms)");
                if feature == READ_ATTRIBUTES {
                    print_attributes(&bytes[16..]);
                }
            }
            Ok(bytes) => println!("  {name} {what}: short, {} bytes ({ms:.2} ms)", bytes.len()),
            Err(code) => println!("  {name} {what}: {code} ({ms:.2} ms)"),
        }
    }
}

fn protocol_query(
    device: HANDLE,
    name: &str,
    protocol: i32,
    data_type: i32,
    value: u32,
    sub: u32,
) -> Option<Vec<u8>> {
    let extra = protocol_data(protocol, data_type, value, sub, 512);
    let query = property_query(StorageDeviceProtocolSpecificProperty.0, &extra);
    let (answer, ms) = ioctl(device, IOCTL_STORAGE_QUERY_PROPERTY, &query, 8 + 40 + 512);
    match answer {
        Ok(bytes) => {
            let offset = u32_at(&bytes, 8 + 16).unwrap_or(0) as usize;
            let length = u32_at(&bytes, 8 + 20).unwrap_or(0) as usize;
            let data = bytes
                .get(8 + offset..8 + offset + length)
                .unwrap_or(&[])
                .to_vec();
            let nonzero = data.iter().filter(|b| **b != 0).count();
            println!("  {name}: ok, {length} bytes, {nonzero} non-zero ({ms:.2} ms)");
            Some(data)
        }
        Err(code) => {
            println!("  {name}: {code} ({ms:.2} ms)");
            None
        }
    }
}

fn ata_protocol(device: HANDLE) {
    let ata = ProtocolTypeAta.0;
    protocol_query(device, "ata identify", ata, AtaDataTypeIdentify.0, 0, 0);
    if let Some(dir) = protocol_query(
        device,
        "ata log directory (0x00)",
        ata,
        AtaDataTypeLogPage.0,
        0,
        0,
    ) {
        let stats_pages = u16_at(&dir, 2 * 0x04);
        println!("    device statistics log pages: {stats_pages:?}");
    }
    if let Some(stats) = protocol_query(
        device,
        "ata device statistics (0x04 page 1)",
        ata,
        AtaDataTypeLogPage.0,
        0x04,
        1,
    ) {
        // Each statistic is a qword whose top byte flags it as supported (bit 63) and valid (bit 62).
        for (at, what) in [
            (8, "power cycles"),
            (16, "power-on hours"),
            (24, "sectors written"),
        ] {
            let word = stats
                .get(at..at + 8)
                .and_then(|s| s.try_into().ok())
                .map(u64::from_le_bytes);
            if let Some(word) = word.filter(|w| w >> 63 == 1) {
                println!("    {what}: {}", word & 0x0000_FFFF_FFFF_FFFF);
            }
        }
    }
}

fn nvme_health(device: HANDLE) {
    let Some(log) = protocol_query(
        device,
        "nvme health log",
        ProtocolTypeNvme.0,
        NVMeDataTypeLogPage.0,
        2,
        0,
    ) else {
        return;
    };
    if log.len() < 176 {
        return;
    }
    let kelvin = u16_at(&log, 1).unwrap_or(0);
    let media_errors = u32_at(&log, 160).unwrap_or(0);
    println!(
        "    critical warning {:#04x}, temp {} C, spare {}% (threshold {}%), used {}%, media errors {media_errors}",
        log[0],
        i32::from(kelvin) - 273,
        log[3],
        log[4],
        log[5]
    );
}

fn management_status(device: HANDLE) {
    let (answer, ms) = ioctl(
        device,
        IOCTL_STORAGE_QUERY_PROPERTY,
        &property_query(StorageDeviceManagementStatus.0, &[]),
        256,
    );
    match answer {
        Ok(bytes) => println!(
            "  management status: health {:?} (0 unknown, 1 unhealthy, 2 warning, 3 healthy), {} status ({ms:.2} ms)",
            u32_at(&bytes, 8),
            u32_at(&bytes, 12).unwrap_or(0)
        ),
        Err(code) => println!("  management status: {code} ({ms:.2} ms)"),
    }
}

fn temperature(device: HANDLE) {
    let (answer, ms) = ioctl(
        device,
        IOCTL_STORAGE_QUERY_PROPERTY,
        &property_query(StorageDeviceTemperatureProperty.0, &[]),
        512,
    );
    match answer {
        Ok(bytes) => {
            let count = u16_at(&bytes, 12).unwrap_or(0) as usize;
            let temps: Vec<i16> = (0..count)
                .filter_map(|i| u16_at(&bytes, 24 + i * 16 + 2).map(|t| t as i16))
                .collect();
            println!(
                "  temperature: {temps:?} C, warning {:?}, critical {:?} ({ms:.2} ms)",
                u16_at(&bytes, 10).map(|t| t as i16),
                u16_at(&bytes, 8).map(|t| t as i16)
            );
        }
        Err(code) => println!("  temperature: {code} ({ms:.2} ms)"),
    }
}
