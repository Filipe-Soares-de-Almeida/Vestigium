use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use serde::Serialize;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Storage::FileSystem::{
    FILE_VER_GET_NEUTRAL, GetFileVersionInfoExW, GetFileVersionInfoSizeExW, VS_FIXEDFILEINFO,
    VerQueryValueW,
};
use windows::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, REG_ROUTINE_FLAGS, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::core::{PCWSTR, w};

/// The first build of Windows 11; its registry still names the product "Windows 10".
const FIRST_WINDOWS_11_BUILD: u32 = 22000;
const CURRENT_VERSION: PCWSTR = w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
/// The signature every `VS_FIXEDFILEINFO` starts with.
const FIXED_SIGNATURE: u32 = 0xFEEF_04BD;

/// Which Windows runs, as the registry tells it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WindowsVersion {
    /// "Windows 11 Pro".
    pub edition: String,
    /// "Windows 11", or the whole edition for a product that is neither 10 nor 11.
    pub family: String,
    /// "24H2"; `None` before Windows 10 20H2.
    pub release: Option<String>,
    /// "26100.2605": the build and its update revision.
    pub build: String,
    /// "10.0.26100.2605", as winver and `ver` print it.
    pub version: String,
}

impl WindowsVersion {
    fn from_values(
        product: &str,
        release: Option<String>,
        (major, minor): (u32, u32),
        build: u32,
        revision: Option<u32>,
    ) -> WindowsVersion {
        let edition = match product.strip_prefix("Windows 10") {
            Some(rest) if build >= FIRST_WINDOWS_11_BUILD => format!("Windows 11{rest}"),
            _ => product.to_string(),
        };
        let family = ["Windows 11", "Windows 10"]
            .into_iter()
            .find(|family| edition.starts_with(family))
            .map_or_else(|| edition.clone(), str::to_string);
        let build = match revision {
            Some(revision) => format!("{build}.{revision}"),
            None => build.to_string(),
        };
        WindowsVersion {
            version: format!("{major}.{minor}.{build}"),
            edition,
            family,
            release: release.filter(|release| !release.is_empty()),
            build,
        }
    }
}

/// The running Windows; `None` when the registry does not say its name or build.
pub fn windows_version() -> Option<WindowsVersion> {
    let product = text(w!("ProductName"))?;
    let build = text(w!("CurrentBuild"))?.trim().parse().ok()?;
    let major = dword(w!("CurrentMajorVersionNumber")).unwrap_or(10);
    let minor = dword(w!("CurrentMinorVersionNumber")).unwrap_or(0);
    Some(WindowsVersion::from_values(
        &product,
        text(w!("DisplayVersion")),
        (major, minor),
        build,
        dword(w!("UBR")),
    ))
}

/// The file version in an executable's version resource, as "major.minor.patch"; `None` when the
/// file is missing or has no version.
pub fn file_version(path: &Path) -> Option<String> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let name = PCWSTR(wide.as_ptr());
    // The neutral resource of the file itself, never the version of a language pack beside it.
    let flags = FILE_VER_GET_NEUTRAL;
    let mut handle = 0u32;
    // SAFETY: a size query on a null-terminated path that lives through the call.
    let size = unsafe { GetFileVersionInfoSizeExW(flags, name, &mut handle) };
    if size == 0 {
        return None;
    }
    let mut block = vec![0u8; size as usize];
    // SAFETY: `block` has the `size` writable bytes the call is told about.
    unsafe { GetFileVersionInfoExW(flags, name, None, size, block.as_mut_ptr().cast()) }.ok()?;
    let mut fixed: *mut c_void = std::ptr::null_mut();
    let mut len = 0u32;
    // SAFETY: `block` holds the version resource filled above; the answer points inside it.
    let found = unsafe { VerQueryValueW(block.as_ptr().cast(), w!(r"\"), &mut fixed, &mut len) };
    if !found.as_bool() || fixed.is_null() || (len as usize) < size_of::<VS_FIXEDFILEINFO>() {
        return None;
    }
    // SAFETY: the pointer names a VS_FIXEDFILEINFO of `len` bytes inside `block`, which is alive.
    let info = unsafe { fixed.cast::<VS_FIXEDFILEINFO>().read_unaligned() };
    (info.dwSignature == FIXED_SIGNATURE)
        .then(|| three_parts(info.dwFileVersionMS, info.dwFileVersionLS))
}

/// "0.1.1" from the two halves of a fixed file version; the fourth part is left out.
fn three_parts(high: u32, low: u32) -> String {
    format!("{}.{}.{}", high >> 16, high & 0xFFFF, low >> 16)
}

fn dword(value: PCWSTR) -> Option<u32> {
    let (mut data, mut size) = (0u32, 4u32);
    // SAFETY: reads one DWORD into a buffer of the size passed; both names are static.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            CURRENT_VERSION,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast()),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

fn text(value: PCWSTR) -> Option<String> {
    let flags = REG_ROUTINE_FLAGS(RRF_RT_REG_SZ.0);
    let mut size = 0u32;
    // SAFETY: a size query with no buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            CURRENT_VERSION,
            value,
            flags,
            None,
            None,
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS || size == 0 {
        return None;
    }
    let mut data = vec![0u16; size as usize / 2 + 1];
    let mut bytes = (data.len() * 2) as u32;
    // SAFETY: `data` has `bytes` writable bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            CURRENT_VERSION,
            value,
            flags,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let end = data.iter().position(|&u| u == 0).unwrap_or(data.len());
    Some(String::from_utf16_lossy(&data[..end])).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_11_is_told_by_its_build_not_its_product_name() {
        let eleven = WindowsVersion::from_values(
            "Windows 10 Pro",
            Some("24H2".into()),
            (10, 0),
            26100,
            Some(2605),
        );
        assert_eq!(
            eleven,
            WindowsVersion {
                edition: "Windows 11 Pro".into(),
                family: "Windows 11".into(),
                release: Some("24H2".into()),
                build: "26100.2605".into(),
                version: "10.0.26100.2605".into(),
            }
        );
        let first = WindowsVersion::from_values("Windows 10 Home", None, (10, 0), 22000, None);
        assert_eq!(first.edition, "Windows 11 Home");
        assert_eq!(first.build, "22000");
    }

    #[test]
    fn windows_10_keeps_its_name() {
        let ten = WindowsVersion::from_values(
            "Windows 10 Pro",
            Some("22H2".into()),
            (10, 0),
            19045,
            Some(5000),
        );
        assert_eq!(ten.edition, "Windows 10 Pro");
        assert_eq!(ten.family, "Windows 10");
        assert_eq!(ten.version, "10.0.19045.5000");
    }

    #[test]
    fn a_product_that_is_neither_10_nor_11_is_its_own_family() {
        let server = WindowsVersion::from_values(
            "Windows Server 2025 Standard",
            Some("24H2".into()),
            (10, 0),
            26100,
            Some(1),
        );
        assert_eq!(server.edition, "Windows Server 2025 Standard");
        assert_eq!(server.family, "Windows Server 2025 Standard");
        let empty = WindowsVersion::from_values(
            "Windows 10 Pro",
            Some(String::new()),
            (10, 0),
            19041,
            None,
        );
        assert_eq!(empty.release, None);
    }

    #[test]
    fn this_pc_reports_its_windows() {
        let found = windows_version().expect("the registry names this Windows");
        assert!(found.family.starts_with("Windows"), "{found:?}");
        assert!(found.version.starts_with("10.0."), "{found:?}");
    }

    #[test]
    fn a_fixed_version_keeps_three_parts() {
        assert_eq!(three_parts(0x0000_0001, 0x0001_0000), "0.1.1");
        assert_eq!(three_parts(0x0002_000C, 0x0005_0007), "2.12.5");
    }

    #[test]
    fn a_system_file_has_a_version_and_a_missing_one_has_none() {
        let windir = std::env::var_os("SystemRoot").unwrap();
        let kernel = Path::new(&windir).join(r"System32\kernel32.dll");
        // Windows may report an older major version of its own files to an unmanifested test.
        let found = file_version(&kernel).unwrap();
        let parts: Vec<u32> = found.split('.').map(|part| part.parse().unwrap()).collect();
        assert_eq!(parts.len(), 3, "{found}");
        assert!(parts[0] >= 6 && parts[2] >= 10_240, "{found}");
        assert_eq!(
            file_version(Path::new(r"C:\Nowhere\Vestigium.Helper.exe")),
            None
        );
    }
}
