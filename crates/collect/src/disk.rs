use std::ffi::c_void;

use vestigium_core::Media;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    DEVICE_SEEK_PENALTY_DESCRIPTOR, IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery,
    STORAGE_PROPERTY_QUERY, StorageDeviceSeekPenaltyProperty,
};
use windows::core::HSTRING;

/// The drive number in a PhysicalDisk instance name: "0 C:" is drive 0; "_Total" is none.
pub fn physical_drive(instance: &str) -> Option<u32> {
    instance.split_whitespace().next()?.parse().ok()
}

/// Whether drive `index` has a seek penalty, asked with no access rights so no elevation is needed.
pub fn disk_media(index: u32) -> Media {
    let path = HSTRING::from(format!(r"\\.\PhysicalDrive{index}"));
    // SAFETY: opens the device for queries only; the handle is closed below.
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
    let Ok(device) = opened else {
        return Media::Unknown;
    };
    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceSeekPenaltyProperty,
        QueryType: PropertyStandardQuery,
        ..Default::default()
    };
    let mut answer = DEVICE_SEEK_PENALTY_DESCRIPTOR::default();
    let mut returned = 0u32;
    // SAFETY: both buffers are live structs of the sizes passed.
    let asked = unsafe {
        DeviceIoControl(
            device,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some((&raw const query).cast::<c_void>()),
            size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            Some((&raw mut answer).cast::<c_void>()),
            size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
            Some(&mut returned),
            None,
        )
    };
    // SAFETY: `device` was opened above and is closed exactly once.
    let _ = unsafe { CloseHandle(device) };
    match asked {
        Ok(()) if returned as usize >= size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() => {
            if answer.IncursSeekPenalty {
                Media::Hdd
            } else {
                Media::Ssd
            }
        }
        _ => Media::Unknown,
    }
}
