use std::ffi::c_void;

use vestigium_core::health::{DiskHealth, NvmeDetail, nvme_detail, nvme_health, smart_attributes};
use vestigium_core::hwtest::Bus;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    BusTypeAta, BusTypeNvme, BusTypeSata, BusTypeUsb, CreateFileW, FILE_FLAGS_AND_ATTRIBUTES,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    IOCTL_STORAGE_PREDICT_FAILURE, IOCTL_STORAGE_QUERY_PROPERTY, NVMeDataTypeLogPage,
    ProtocolTypeNvme, StorageDeviceProperty, StorageDeviceProtocolSpecificProperty,
};
use windows::core::HSTRING;

/// NVMe log page of SMART / Health Information.
const NVME_HEALTH_LOG: u32 = 2;
const LOG_LEN: usize = 512;
/// Size of STORAGE_PROTOCOL_SPECIFIC_DATA, which precedes the log in the query and the answer.
const PROTOCOL_DATA_LEN: usize = 40;
/// Offset of BusType in STORAGE_DEVICE_DESCRIPTOR.
const BUS_TYPE_AT: usize = 28;

/// A physical drive opened with no access rights, which the health queries need and which
/// needs no elevation.
struct Drive(HANDLE);

impl Drive {
    fn open(index: u32) -> Option<Drive> {
        let path = HSTRING::from(format!(r"\\.\PhysicalDrive{index}"));
        // SAFETY: opens the device for queries only; `Drop` closes the handle.
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
        opened.ok().map(Drive)
    }

    /// The bytes a control code wrote back, or `None` when the driver refused it.
    fn ask(&self, code: u32, input: &[u8], out_len: usize) -> Option<Vec<u8>> {
        let mut out = vec![0u8; out_len];
        let mut returned = 0u32;
        // SAFETY: both buffers are live for the call and their lengths are the ones passed.
        unsafe {
            DeviceIoControl(
                self.0,
                code,
                (!input.is_empty()).then_some(input.as_ptr().cast::<c_void>()),
                input.len() as u32,
                Some(out.as_mut_ptr().cast::<c_void>()),
                out.len() as u32,
                Some(&mut returned),
                None,
            )
        }
        .ok()?;
        out.truncate(returned as usize);
        Some(out)
    }
}

impl Drop for Drive {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by `Drive::open` and is closed once, here.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// A standard STORAGE_PROPERTY_QUERY for `property`, followed by `extra` parameters.
fn property_query(property: i32, extra: &[u8]) -> Vec<u8> {
    let mut query = Vec::with_capacity(8 + extra.len().max(4));
    query.extend_from_slice(&property.to_le_bytes());
    query.extend_from_slice(&0i32.to_le_bytes());
    if extra.is_empty() {
        query.extend_from_slice(&[0; 4]);
    } else {
        query.extend_from_slice(extra);
    }
    query
}

fn bus_type(drive: &Drive) -> Option<i32> {
    drive
        .ask(
            IOCTL_STORAGE_QUERY_PROPERTY,
            &property_query(StorageDeviceProperty.0, &[]),
            1024,
        )
        .and_then(|d| {
            Some(i32::from_le_bytes(
                d.get(BUS_TYPE_AT..BUS_TYPE_AT + 4)?.try_into().ok()?,
            ))
        })
}

fn is_nvme(drive: &Drive) -> bool {
    bus_type(drive) == Some(BusTypeNvme.0)
}

/// How physical drive `index` is attached; a RAID controller or an unknown answer is `Other`.
pub fn disk_bus(index: u32) -> Bus {
    match Drive::open(index).and_then(|drive| bus_type(&drive)) {
        Some(t) if t == BusTypeSata.0 || t == BusTypeAta.0 => Bus::Sata,
        Some(t) if t == BusTypeNvme.0 => Bus::Nvme,
        Some(t) if t == BusTypeUsb.0 => Bus::Usb,
        _ => Bus::Other,
    }
}

/// The whole NVMe health log of physical drive `index`; `None` for any other disk.
pub fn disk_nvme(index: u32) -> Option<NvmeDetail> {
    let drive = Drive::open(index)?;
    is_nvme(&drive)
        .then(|| nvme_log(&drive))
        .flatten()
        .and_then(|log| nvme_detail(&log))
}

/// The NVMe health log page, asked through the storage stack rather than a pass-through.
fn nvme_log(drive: &Drive) -> Option<Vec<u8>> {
    let fields: [u32; 10] = [
        ProtocolTypeNvme.0 as u32,
        NVMeDataTypeLogPage.0 as u32,
        NVME_HEALTH_LOG,
        0,
        PROTOCOL_DATA_LEN as u32,
        LOG_LEN as u32,
        0,
        0,
        0,
        0,
    ];
    let mut extra: Vec<u8> = fields.iter().flat_map(|f| f.to_le_bytes()).collect();
    extra.resize(PROTOCOL_DATA_LEN + LOG_LEN, 0);
    let query = property_query(StorageDeviceProtocolSpecificProperty.0, &extra);
    let answer = drive.ask(
        IOCTL_STORAGE_QUERY_PROPERTY,
        &query,
        8 + PROTOCOL_DATA_LEN + LOG_LEN,
    )?;
    // The answer is a version and a size, then the protocol data with its own offset to the log.
    let offset = u32::from_le_bytes(answer.get(24..28)?.try_into().ok()?) as usize;
    answer
        .get(8 + offset..8 + offset + LOG_LEN)
        .map(<[u8]>::to_vec)
}

/// SMART attributes and the disk's own verdict, or the NVMe health log; `None` when the drive
/// answers neither.
pub fn disk_health(index: u32, name: &str, now_ms: i64) -> Option<DiskHealth> {
    let drive = Drive::open(index)?;
    let nvme = is_nvme(&drive)
        .then(|| nvme_log(&drive))
        .flatten()
        .and_then(|log| nvme_health(&log));
    let predicted = drive.ask(IOCTL_STORAGE_PREDICT_FAILURE, &[], 4 + 512);
    if predicted.is_none() && nvme.is_none() {
        return None;
    }
    let (predict_failure, attributes) = match predicted.as_deref() {
        Some([a, b, c, d, rest @ ..]) => (
            u32::from_le_bytes([*a, *b, *c, *d]) != 0,
            // An NVMe disk's vendor block is not an ATA attribute table.
            if nvme.is_none() {
                smart_attributes(rest)
            } else {
                Vec::new()
            },
        ),
        _ => (false, Vec::new()),
    };
    Some(DiskHealth {
        name: name.to_string(),
        read_ms: now_ms,
        predict_failure,
        attributes,
        nvme,
    })
}
