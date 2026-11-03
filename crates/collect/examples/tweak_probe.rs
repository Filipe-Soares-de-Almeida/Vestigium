//! Reads, without elevation, the state of each Windows setting Vestigium may offer to change, with
//! what each read costs: `tweak_probe [--try-power]`. `--try-power` writes the current ASPM value
//! back to the active power scheme, to learn whether a scheme write needs elevation.

use std::ffi::c_void;
use std::time::Instant;

use windows::Win32::Foundation::{HLOCAL, LocalFree, WIN32_ERROR};
use windows::Win32::Storage::FileSystem::{
    GetFileAttributesExW, GetFileExInfoStandard, WIN32_FILE_ATTRIBUTE_DATA,
};
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection,
    EvtRender, EvtRenderEventXml,
};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, PowerGetActiveScheme, PowerReadACValueIndex, PowerReadFriendlyName,
    PowerSetActiveScheme, PowerWriteACValueIndex, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_VALUE_TYPE, RRF_RT_REG_DWORD,
    RegCloseKey, RegEnumValueW, RegGetValueW, RegOpenKeyExW,
};
use windows::Win32::System::Services::{
    CloseServiceHandle, OpenSCManagerW, OpenServiceW, QUERY_SERVICE_CONFIGW, QueryServiceConfig2W,
    QueryServiceConfigW, QueryServiceStatus, SC_MANAGER_CONNECT,
    SERVICE_CONFIG_DELAYED_AUTO_START_INFO, SERVICE_DELAYED_AUTO_START_INFO, SERVICE_QUERY_CONFIG,
    SERVICE_QUERY_STATUS, SERVICE_STATUS,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};
use windows::core::{GUID, HSTRING, PCWSTR, PWSTR};

const SUB_PCIEXPRESS: GUID = GUID::from_u128(0x501a4d13_42af_4429_9fd1_a8218c268e20);
const ASPM: GUID = GUID::from_u128(0xee12f906_d277_404b_b6da_e5fa1a576df5);
const SUB_USB: GUID = GUID::from_u128(0x2a737441_1930_4402_8d77_b2bebba308a3);
const USB_SUSPEND: GUID = GUID::from_u128(0x48e6b7a6_50f5_4782_a5d4_53bb8f07e226);

fn main() {
    let try_power = std::env::args().any(|a| a == "--try-power");
    registry();
    services();
    power(try_power);
    hibernation_file();
    startup();
    machine();
    restore_points();
}

fn ms_since(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn dword(hive: HKEY, key: &str, value: &str) -> Result<u32, WIN32_ERROR> {
    let mut data = 0u32;
    let mut size = 4u32;
    // SAFETY: a DWORD read into a 4-byte buffer whose size is passed.
    let status = unsafe {
        RegGetValueW(
            hive,
            &HSTRING::from(key),
            &HSTRING::from(value),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast::<c_void>()),
            Some(&mut size),
        )
    };
    if status.is_ok() {
        Ok(data)
    } else {
        Err(status)
    }
}

fn registry() {
    const READS: [(&str, &str, &str); 11] = [
        (
            "HKLM",
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Power",
            "HiberbootEnabled",
        ),
        (
            "HKLM",
            r"SYSTEM\CurrentControlSet\Control\Power",
            "HibernateEnabled",
        ),
        (
            "HKLM",
            r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            "HwSchMode",
        ),
        ("HKLM", r"SOFTWARE\Microsoft\Windows\Dwm", "OverlayTestMode"),
        (
            "HKLM",
            r"SYSTEM\CurrentControlSet\Control\CrashControl",
            "CrashDumpEnabled",
        ),
        (
            "HKLM",
            r"SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps",
            "DumpType",
        ),
        (
            "HKLM",
            r"SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps",
            "DumpCount",
        ),
        (
            "HKCU",
            r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
            "AppCaptureEnabled",
        ),
        (
            "HKCU",
            r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
            "HistoricalCaptureEnabled",
        ),
        ("HKCU", r"System\GameConfigStore", "GameDVR_Enabled"),
        (
            "HKLM",
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore",
            "SystemRestorePointCreationFrequency",
        ),
    ];
    println!("== registry");
    for (hive, key, value) in READS {
        let root = if hive == "HKLM" {
            HKEY_LOCAL_MACHINE
        } else {
            HKEY_CURRENT_USER
        };
        let started = Instant::now();
        let read = dword(root, key, value);
        let shown = match read {
            Ok(v) => format!("{v}"),
            Err(e) => format!("absent or unreadable ({})", e.0),
        };
        println!(
            "{hive}\\{key} {value} = {shown} ({:.3} ms)",
            ms_since(started)
        );
    }
}

fn services() {
    println!("== services");
    let started = Instant::now();
    // SAFETY: a connection to the local service manager, closed below.
    let scm = match unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) } {
        Ok(h) => h,
        Err(e) => return println!("OpenSCManager: {e}"),
    };
    for name in ["SysMain", "WSearch"] {
        // SAFETY: opens one service for reading its configuration and status; closed below.
        let service = match unsafe {
            OpenServiceW(
                scm,
                &HSTRING::from(name),
                SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS,
            )
        } {
            Ok(h) => h,
            Err(e) => {
                println!("{name}: open failed: {e}");
                continue;
            }
        };
        let mut buf = vec![0u64; 1024];
        let mut needed = 0u32;
        // SAFETY: the buffer has the writable length passed, aligned for the struct.
        let config = unsafe {
            QueryServiceConfigW(
                service,
                Some(buf.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>()),
                8192,
                &mut needed,
            )
        };
        let start = if config.is_ok() {
            // SAFETY: the call succeeded and filled the struct at the start of the buffer.
            unsafe {
                (*buf.as_ptr().cast::<QUERY_SERVICE_CONFIGW>())
                    .dwStartType
                    .0
            }
        } else {
            u32::MAX
        };
        let mut delayed = SERVICE_DELAYED_AUTO_START_INFO::default();
        let mut size = 0u32;
        // SAFETY: the struct has the size passed.
        let _ = unsafe {
            QueryServiceConfig2W(
                service,
                SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
                Some(std::slice::from_raw_parts_mut(
                    (&raw mut delayed).cast::<u8>(),
                    size_of::<SERVICE_DELAYED_AUTO_START_INFO>(),
                )),
                &mut size,
            )
        };
        let mut status = SERVICE_STATUS::default();
        // SAFETY: the handle has SERVICE_QUERY_STATUS.
        let state =
            unsafe { QueryServiceStatus(service, &mut status) }.map(|()| status.dwCurrentState.0);
        println!(
            "{name}: config {:?}, start type {start}, delayed {}, state {:?}",
            config.map(|()| "ok"),
            delayed.fDelayedAutostart.as_bool(),
            state
        );
        // SAFETY: closes the handle opened above.
        let _ = unsafe { CloseServiceHandle(service) };
    }
    // SAFETY: closes the manager opened above.
    let _ = unsafe { CloseServiceHandle(scm) };
    println!("services read in {:.3} ms", ms_since(started));
}

fn power(try_power: bool) {
    println!("== power");
    let started = Instant::now();
    let mut scheme: *mut GUID = std::ptr::null_mut();
    // SAFETY: Windows allocates the GUID, freed with LocalFree below.
    let status = unsafe { PowerGetActiveScheme(None, &mut scheme) };
    if status.is_err() || scheme.is_null() {
        return println!("PowerGetActiveScheme: {}", status.0);
    }
    // SAFETY: the pointer came from a successful call.
    let active = unsafe { *scheme };
    let mut name = [0u16; 128];
    let mut size = (name.len() * 2) as u32;
    // SAFETY: the buffer has the byte length passed.
    let _ = unsafe {
        PowerReadFriendlyName(
            None,
            Some(&active),
            None,
            None,
            Some(name.as_mut_ptr().cast()),
            &mut size,
        )
    };
    let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    println!(
        "active scheme {active:?} \"{}\"",
        String::from_utf16_lossy(&name[..len])
    );
    let mut values = Vec::new();
    for (label, sub, setting) in [
        ("ASPM (AC)", SUB_PCIEXPRESS, ASPM),
        ("USB selective suspend (AC)", SUB_USB, USB_SUSPEND),
    ] {
        let mut value = 0u32;
        // SAFETY: reads one AC value of the active scheme into a u32.
        let read = unsafe {
            PowerReadACValueIndex(None, Some(&active), Some(&sub), Some(&setting), &mut value)
        };
        println!("{label} = {value} (status {})", read.0);
        values.push((sub, setting, value, read.0 == 0));
    }
    println!("power read in {:.3} ms", ms_since(started));
    if try_power && let Some(&(sub, setting, value, true)) = values.first() {
        // SAFETY: writes back the value just read; nothing changes.
        let write =
            unsafe { PowerWriteACValueIndex(None, &active, Some(&sub), Some(&setting), value) };
        // SAFETY: re-applies the scheme that is already active.
        let apply = unsafe { PowerSetActiveScheme(None, Some(&active)) };
        println!(
            "write same ASPM value without admin: write {}, apply {}",
            write.0, apply.0
        );
    }
    // SAFETY: frees the GUID Windows allocated.
    unsafe { LocalFree(Some(HLOCAL(scheme.cast()))) };
}

fn hibernation_file() {
    println!("== hiberfil.sys");
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let mut data = WIN32_FILE_ATTRIBUTE_DATA::default();
    let started = Instant::now();
    // SAFETY: the struct matches GetFileExInfoStandard.
    let read = unsafe {
        GetFileAttributesExW(
            &HSTRING::from(format!(r"{drive}\hiberfil.sys")),
            GetFileExInfoStandard,
            (&raw mut data).cast::<c_void>(),
        )
    };
    let size = (u64::from(data.nFileSizeHigh) << 32) | u64::from(data.nFileSizeLow);
    println!(
        "{read:?}: {} MB ({:.3} ms)",
        size / (1024 * 1024),
        ms_since(started)
    );
}

fn startup() {
    println!("== startup entries (names only shown as counts; bytes of StartupApproved)");
    let places = [
        (
            "HKCU",
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Run",
        ),
        (
            "HKLM",
            HKEY_LOCAL_MACHINE,
            r"Software\Microsoft\Windows\CurrentVersion\Run",
        ),
        (
            "HKLM",
            HKEY_LOCAL_MACHINE,
            r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
        ),
        (
            "HKCU",
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        ),
        (
            "HKCU",
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
        ),
        (
            "HKLM",
            HKEY_LOCAL_MACHINE,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        ),
        (
            "HKLM",
            HKEY_LOCAL_MACHINE,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32",
        ),
    ];
    for (hive, root, key) in places {
        let values = enum_values(root, key);
        let approved = key.contains("StartupApproved");
        println!("{hive}\\{key}: {} values", values.len());
        if approved {
            for (i, (_, kind, data)) in values.iter().enumerate() {
                let hex: Vec<String> = data.iter().map(|b| format!("{b:02x}")).collect();
                println!(
                    "  #{i}: type {kind}, {} bytes: {}",
                    data.len(),
                    hex.join(" ")
                );
            }
        }
    }
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let folder = format!(r"{appdata}\Microsoft\Windows\Start Menu\Programs\Startup");
    let count = std::fs::read_dir(&folder).map(|d| d.count()).unwrap_or(0);
    println!("user Startup folder: {count} entries");
}

fn enum_values(root: HKEY, key: &str) -> Vec<(String, u32, Vec<u8>)> {
    let mut opened = HKEY::default();
    // SAFETY: opens a key for reading; closed below.
    if unsafe { RegOpenKeyExW(root, &HSTRING::from(key), None, KEY_READ, &mut opened) }.is_err() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for index in 0.. {
        let mut name = [0u16; 512];
        let mut name_len = name.len() as u32;
        let mut kind = REG_VALUE_TYPE::default();
        let mut data = vec![0u8; 4096];
        let mut data_len = data.len() as u32;
        // SAFETY: both buffers have the lengths passed.
        let status = unsafe {
            RegEnumValueW(
                opened,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                Some(&mut kind.0),
                Some(data.as_mut_ptr()),
                Some(&mut data_len),
            )
        };
        if status.is_err() {
            break;
        }
        data.truncate(data_len as usize);
        out.push((
            String::from_utf16_lossy(&name[..name_len as usize]),
            kind.0,
            data,
        ));
    }
    // SAFETY: closes the key opened above.
    let _ = unsafe { RegCloseKey(opened) };
    out
}

fn machine() {
    println!("== machine");
    // SAFETY: plain metric query.
    let monitors = unsafe { GetSystemMetrics(SM_CMONITORS) };
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: the struct is the one the call fills.
    let read = unsafe { GetSystemPowerStatus(&mut status) };
    println!(
        "monitors {monitors}; power {read:?}: AC {}, battery flag {}",
        status.ACLineStatus, status.BatteryFlag
    );
}

/// The newest "System Restore" 8194 events in the Application log, as their XML time.
fn restore_points() {
    println!("== restore points (Application, System Restore 8194)");
    let started = Instant::now();
    let query = "*[System[Provider[@Name='System Restore'] and EventID=8194]]";
    // SAFETY: a reverse query over one channel; closed below.
    let results = match unsafe {
        EvtQuery(
            None,
            &HSTRING::from("Application"),
            &HSTRING::from(query),
            EvtQueryChannelPath.0 | EvtQueryReverseDirection.0,
        )
    } {
        Ok(h) => h,
        Err(e) => return println!("EvtQuery: {e}"),
    };
    let mut batch = [0isize; 3];
    let mut returned = 0u32;
    // SAFETY: `batch` receives up to 3 event handles, each closed below.
    let _ = unsafe { EvtNext(results, &mut batch, 2000, 0, &mut returned) };
    for raw in &batch[..returned as usize] {
        let event = EVT_HANDLE(*raw);
        let mut buf = vec![0u16; 8192];
        let mut used = 0u32;
        let mut count = 0u32;
        // SAFETY: the buffer has the byte length passed.
        let ok = unsafe {
            EvtRender(
                None,
                event,
                EvtRenderEventXml.0,
                (buf.len() * 2) as u32,
                Some(buf.as_mut_ptr().cast()),
                &mut used,
                &mut count,
            )
        };
        if ok.is_ok() {
            let xml = String::from_utf16_lossy(&buf[..(used as usize / 2).saturating_sub(1)]);
            let time = xml
                .split("SystemTime='")
                .nth(1)
                .and_then(|s| s.split('\'').next())
                .unwrap_or("?");
            let record = xml
                .split("<EventRecordID>")
                .nth(1)
                .and_then(|s| s.split('<').next())
                .unwrap_or("?");
            println!("8194 at {time} (record {record})");
        }
        // SAFETY: closes the event handle.
        let _ = unsafe { EvtClose(event) };
    }
    // SAFETY: closes the query handle.
    let _ = unsafe { EvtClose(results) };
    println!("{returned} found in {:.3} ms", ms_since(started));
}
