mod disks;
mod security;
mod system;

use vestigium_core::health::{DiskHealth, PageFile, SecurityProduct, VolumeSpace};

pub use disks::{disk_bus, disk_health, disk_nvme, disk_size};
pub use security::antivirus;
pub use system::{file_system, page_files, system_drive, volume_drive, volumes};

/// What a health pass reads from Windows, so the agent's tests can stand in for the machine.
pub trait HealthSource {
    /// SMART or NVMe health of physical drive `index`, kept under the counters' `name` for it.
    fn disk(&mut self, index: u32, name: &str, now_ms: i64) -> Option<DiskHealth>;
    fn volumes(&mut self) -> Vec<VolumeSpace>;
    /// Number of the physical drive that holds Windows.
    fn system_drive(&mut self) -> Option<u32>;
    fn page_files(&mut self) -> Option<Vec<PageFile>>;
    fn antivirus(&mut self) -> Option<Vec<SecurityProduct>>;
}

/// The live machine, read without elevation.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsHealth;

impl HealthSource for WindowsHealth {
    fn disk(&mut self, index: u32, name: &str, now_ms: i64) -> Option<DiskHealth> {
        disk_health(index, name, now_ms)
    }

    fn volumes(&mut self) -> Vec<VolumeSpace> {
        volumes()
    }

    fn system_drive(&mut self) -> Option<u32> {
        system_drive()
    }

    fn page_files(&mut self) -> Option<Vec<PageFile>> {
        page_files()
    }

    fn antivirus(&mut self) -> Option<Vec<SecurityProduct>> {
        antivirus()
    }
}
