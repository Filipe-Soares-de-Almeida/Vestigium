use std::path::Path;

use vestigium_core::{
    ERROR_MAY_HAVE_CHANGED, Guid, Hive, Setting, SettingStore, StartupEntry, StartupList, Value,
};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_DIR_NOT_EMPTY, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HANDLE, HLOCAL, LUID,
    LocalFree, STATUS_SUCCESS, WIN32_ERROR,
};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::Storage::FileSystem::{FindClose, FindFirstFileW, WIN32_FIND_DATAW};
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryForwardDirection,
    EvtRenderEventXml,
};
use windows::Win32::System::Power::{
    ACCESS_SCHEME, CallNtPowerInformation, GetSystemPowerStatus, PowerEnumerate,
    PowerGetActiveScheme, PowerReadACValueIndex, PowerSetActiveScheme, PowerWriteACValueIndex,
    SYSTEM_POWER_CAPABILITIES, SYSTEM_POWER_STATUS, SystemPowerCapabilities,
    SystemReserveHiberFile,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_BINARY, REG_DWORD,
    REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE, RRF_RT_REG_BINARY,
    RRF_RT_REG_DWORD, RegCloseKey, RegCreateKeyExW, RegDeleteKeyValueW, RegDeleteKeyW,
    RegEnumValueW, RegGetValueW, RegOpenKeyExW, RegQueryInfoKeyW, RegSetKeyValueW,
};
use windows::Win32::System::Services::{
    ChangeServiceConfig2W, ChangeServiceConfigW, CloseServiceHandle, ControlService,
    ENUM_SERVICE_TYPE, OpenSCManagerW, OpenServiceW, QUERY_SERVICE_CONFIGW, QueryServiceConfig2W,
    QueryServiceConfigW, QueryServiceStatus, SC_HANDLE, SC_MANAGER_CONNECT, SERVICE_CHANGE_CONFIG,
    SERVICE_CONFIG_DELAYED_AUTO_START_INFO, SERVICE_CONTROL_STOP, SERVICE_DELAYED_AUTO_START_INFO,
    SERVICE_ERROR, SERVICE_NO_CHANGE, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, SERVICE_RUNNING,
    SERVICE_START, SERVICE_START_TYPE, SERVICE_STATUS, SERVICE_STOP, StartServiceW,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};
use windows::core::{GUID, HSTRING, PCWSTR, PWSTR};

use crate::CollectError;
use crate::events::render_text;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN32_KEY: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
/// Where System Restore logs a point it created (8194) or skipped as recent enough (8216).
pub const RESTORE_CREATED: u16 = 8194;
pub const RESTORE_SKIPPED: u16 = 8216;

fn root(hive: Hive) -> HKEY {
    match hive {
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
        Hive::CurrentUser => HKEY_CURRENT_USER,
    }
}

/// What a setting holds now, read without elevation; `None` when it cannot be read here.
pub fn read_setting(setting: &Setting) -> Option<Value> {
    match setting {
        Setting::Dword { hive, key, name } => read_dword(root(*hive), key, name),
        Setting::Key { hive, key } => {
            let mut opened = HKEY::default();
            // SAFETY: opens a key for reading; closed right after.
            let status = unsafe {
                RegOpenKeyExW(
                    root(*hive),
                    &HSTRING::from(key.as_str()),
                    None,
                    KEY_READ,
                    &mut opened,
                )
            };
            if status == ERROR_SUCCESS {
                // SAFETY: closes the key opened above.
                let _ = unsafe { RegCloseKey(opened) };
                Some(Value::Present)
            } else {
                (status == ERROR_FILE_NOT_FOUND).then_some(Value::Absent)
            }
        }
        Setting::Service { name } => read_service(name),
        Setting::PowerAc {
            subgroup,
            setting,
            scheme,
        } => {
            let scheme = scheme_of(*scheme)?;
            let mut value = 0u32;
            // SAFETY: reads one AC value of one scheme into a u32.
            let status = unsafe {
                PowerReadACValueIndex(
                    None,
                    Some(&GUID::from_u128(scheme.0)),
                    Some(&GUID::from_u128(subgroup.0)),
                    Some(&GUID::from_u128(setting.0)),
                    &mut value,
                )
            };
            (status == ERROR_SUCCESS).then_some(Value::Index(value))
        }
        Setting::ActiveScheme => active_scheme().map(Value::Scheme),
        Setting::Hibernation => {
            let mut caps = SYSTEM_POWER_CAPABILITIES::default();
            // SAFETY: the output buffer is the struct the information level fills.
            let status = unsafe {
                CallNtPowerInformation(
                    SystemPowerCapabilities,
                    None,
                    0,
                    Some((&raw mut caps).cast()),
                    size_of::<SYSTEM_POWER_CAPABILITIES>() as u32,
                )
            };
            (status == STATUS_SUCCESS).then_some(Value::On(caps.HiberFilePresent))
        }
        Setting::StartupApproval { hive, list, name } => {
            let key = HSTRING::from(list.approval_key());
            let mut data = [0u8; 64];
            let mut size = data.len() as u32;
            // SAFETY: reads a binary value into a buffer of the size passed.
            let status = unsafe {
                RegGetValueW(
                    root(*hive),
                    &key,
                    &HSTRING::from(name.as_str()),
                    RRF_RT_REG_BINARY,
                    None,
                    Some(data.as_mut_ptr().cast()),
                    Some(&mut size),
                )
            };
            if status == ERROR_SUCCESS {
                Some(Value::Bytes(data[..size as usize].to_vec()))
            } else {
                (status == ERROR_FILE_NOT_FOUND).then_some(Value::Absent)
            }
        }
    }
}

fn read_dword(hive: HKEY, key: &str, name: &str) -> Option<Value> {
    let (mut data, mut size) = (0u32, 4u32);
    // SAFETY: reads one DWORD into a buffer of the size passed.
    let status = unsafe {
        RegGetValueW(
            hive,
            &HSTRING::from(key),
            &HSTRING::from(name),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast()),
            Some(&mut size),
        )
    };
    if status == ERROR_SUCCESS {
        Some(Value::Dword(data))
    } else {
        (status == ERROR_FILE_NOT_FOUND).then_some(Value::Absent)
    }
}

/// A service handle, closed when dropped.
struct Service(SC_HANDLE);

impl Drop for Service {
    fn drop(&mut self) {
        // SAFETY: the handle came from the service manager and is closed once, here.
        let _ = unsafe { CloseServiceHandle(self.0) };
    }
}

fn read_service(name: &str) -> Option<Value> {
    // SAFETY: a connection to the local service manager; closed by `Service`.
    let manager = Service(unsafe { OpenSCManagerW(None, None, SC_MANAGER_CONNECT) }.ok()?);
    // SAFETY: opens one service for reading its configuration and status; closed by `Service`.
    let service = Service(
        unsafe {
            OpenServiceW(
                manager.0,
                &HSTRING::from(name),
                SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS,
            )
        }
        .ok()?,
    );
    let mut buf = vec![0u64; 1024];
    let mut needed = 0u32;
    // SAFETY: the buffer has the writable length passed, aligned for the struct.
    unsafe {
        QueryServiceConfigW(
            service.0,
            Some(buf.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>()),
            (buf.len() * 8) as u32,
            &mut needed,
        )
    }
    .ok()?;
    // SAFETY: the call succeeded and filled the struct at the start of the buffer.
    let start = unsafe {
        (*buf.as_ptr().cast::<QUERY_SERVICE_CONFIGW>())
            .dwStartType
            .0
    };
    let mut delayed = SERVICE_DELAYED_AUTO_START_INFO::default();
    let mut size = 0u32;
    // SAFETY: the slice covers exactly the struct the information level fills.
    let delayed = unsafe {
        QueryServiceConfig2W(
            service.0,
            SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
            Some(std::slice::from_raw_parts_mut(
                (&raw mut delayed).cast::<u8>(),
                size_of::<SERVICE_DELAYED_AUTO_START_INFO>(),
            )),
            &mut size,
        )
    }
    .is_ok()
        && delayed.fDelayedAutostart.as_bool();
    let mut status = SERVICE_STATUS::default();
    // SAFETY: the handle has SERVICE_QUERY_STATUS.
    unsafe { QueryServiceStatus(service.0, &mut status) }.ok()?;
    Some(Value::Service {
        start,
        delayed,
        running: status.dwCurrentState == SERVICE_RUNNING,
    })
}

/// Writes one plugged-in value of one power scheme.
fn write_ac_index(scheme: Guid, subgroup: Guid, setting: Guid, index: u32) -> Result<(), u32> {
    // SAFETY: writes one AC value of one existing scheme.
    ok(unsafe {
        PowerWriteACValueIndex(
            None,
            &GUID::from_u128(scheme.0),
            Some(&GUID::from_u128(subgroup.0)),
            Some(&GUID::from_u128(setting.0)),
            index,
        )
    })
}

/// The scheme a power value is read from or written to: the one named when it still exists,
/// otherwise the active one when none is named.
fn scheme_of(named: Option<Guid>) -> Option<Guid> {
    match named {
        Some(scheme) => scheme_exists(scheme).then_some(scheme),
        None => active_scheme(),
    }
}

/// Whether Windows still lists a power scheme with this GUID.
fn scheme_exists(wanted: Guid) -> bool {
    for index in 0.. {
        let mut found = GUID::default();
        let mut size = size_of::<GUID>() as u32;
        // SAFETY: the buffer is a GUID and its size is passed with it.
        let status = unsafe {
            PowerEnumerate(
                None,
                None,
                None,
                ACCESS_SCHEME,
                index,
                Some((&raw mut found).cast()),
                &mut size,
            )
        };
        if status != ERROR_SUCCESS {
            return false;
        }
        if found.to_u128() == wanted.0 {
            return true;
        }
    }
    false
}

fn active_scheme() -> Option<Guid> {
    let mut scheme: *mut GUID = std::ptr::null_mut();
    // SAFETY: Windows allocates the GUID, freed with LocalFree below.
    let status = unsafe { PowerGetActiveScheme(None, &mut scheme) };
    if status != ERROR_SUCCESS || scheme.is_null() {
        return None;
    }
    // SAFETY: the pointer came from a successful call and is freed once.
    let guid = unsafe {
        let guid = *scheme;
        let _ = LocalFree(Some(HLOCAL(scheme.cast())));
        guid
    };
    Some(Guid(guid.to_u128()))
}

/// The programs Windows starts at sign-in from the Run keys and the Startup folders.
pub fn startup_entries() -> Vec<StartupEntry> {
    let mut out = Vec::new();
    for (hive, list, key) in [
        (Hive::CurrentUser, StartupList::Run, RUN_KEY),
        (Hive::LocalMachine, StartupList::Run, RUN_KEY),
        (Hive::LocalMachine, StartupList::Run32, RUN32_KEY),
    ] {
        for (name, command) in run_values(root(hive), key) {
            out.push(StartupEntry {
                hive,
                list,
                name,
                exe: exe_of(&command),
            });
        }
    }
    let folders = [
        (Hive::CurrentUser, std::env::var("APPDATA").ok()),
        (Hive::LocalMachine, std::env::var("ProgramData").ok()),
    ];
    for (hive, base) in folders {
        let Some(base) = base else { continue };
        let folder = Path::new(&base).join(r"Microsoft\Windows\Start Menu\Programs\Startup");
        let Ok(entries) = std::fs::read_dir(folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.eq_ignore_ascii_case("desktop.ini") {
                continue;
            }
            let exe = Path::new(&name)
                .file_stem()
                .map_or_else(|| name.clone(), |s| s.to_string_lossy().into_owned());
            out.push(StartupEntry {
                hive,
                list: StartupList::StartupFolder,
                name,
                exe,
            });
        }
    }
    out
}

/// The string values of a Run key: entry name and command line.
fn run_values(hive: HKEY, key: &str) -> Vec<(String, String)> {
    let mut opened = HKEY::default();
    // SAFETY: opens a key for reading; closed below.
    if unsafe { RegOpenKeyExW(hive, &HSTRING::from(key), None, KEY_READ, &mut opened) }
        != ERROR_SUCCESS
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    for index in 0..1024 {
        let mut name = [0u16; 512];
        let mut name_len = name.len() as u32;
        let mut kind = REG_VALUE_TYPE::default();
        let mut data = vec![0u16; 2048];
        let mut data_len = (data.len() * 2) as u32;
        // SAFETY: both buffers have the lengths passed.
        let status = unsafe {
            RegEnumValueW(
                opened,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                Some(&mut kind.0),
                Some(data.as_mut_ptr().cast()),
                Some(&mut data_len),
            )
        };
        if status != ERROR_SUCCESS {
            break;
        }
        if kind != REG_SZ && kind != REG_EXPAND_SZ {
            continue;
        }
        let text = &data[..(data_len as usize / 2).min(data.len())];
        let text = text.split(|&c| c == 0).next().unwrap_or_default();
        out.push((
            String::from_utf16_lossy(&name[..name_len as usize]),
            String::from_utf16_lossy(text),
        ));
    }
    // SAFETY: closes the key opened above.
    let _ = unsafe { RegCloseKey(opened) };
    out
}

/// The program's file name in a command line, quoted or not: `"C:\A B\App.exe" -x` is "App.exe".
pub fn exe_of(command: &str) -> String {
    let command = command.trim();
    let path = if let Some(rest) = command.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else if let Some(at) = command.to_ascii_lowercase().find(".exe") {
        &command[..at + 4]
    } else {
        command.split_whitespace().next().unwrap_or(command)
    };
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

/// Size of the hibernation file, from the folder listing: the file itself is held open by Windows.
pub fn hiberfile_bytes() -> Option<u64> {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let mut found = WIN32_FIND_DATAW::default();
    // SAFETY: lists one path into the struct; the search handle is closed right after.
    let handle =
        unsafe { FindFirstFileW(&HSTRING::from(format!(r"{drive}\hiberfil.sys")), &mut found) }
            .ok()?;
    // SAFETY: closes the search handle opened above.
    let _ = unsafe { FindClose(handle) };
    Some((u64::from(found.nFileSizeHigh) << 32) | u64::from(found.nFileSizeLow))
}

/// How many monitors are on the desktop.
pub fn monitor_count() -> u32 {
    // SAFETY: a plain metric query.
    u32::try_from(unsafe { GetSystemMetrics(SM_CMONITORS) }).unwrap_or(0)
}

/// Whether the PC has a battery, as a laptop does.
pub fn has_battery() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: the struct is the one the call fills.
    if unsafe { GetSystemPowerStatus(&mut status) }.is_err() {
        return false;
    }
    // 128 means no system battery and 255 that Windows cannot tell.
    status.BatteryFlag != 128 && status.BatteryFlag != 255
}

/// One System Restore entry of the Application log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreEvent {
    pub ts_ms: i64,
    pub id: u16,
    /// The description the requester gave the point.
    pub description: String,
}

/// System Restore's created and skipped entries since `since_ms`, oldest first.
pub fn restore_events(since_ms: i64) -> Result<Vec<RestoreEvent>, CollectError> {
    let query = format!(
        "*[System[Provider[@Name='System Restore'] and (EventID={RESTORE_CREATED} or EventID={RESTORE_SKIPPED}) \
         and TimeCreated[@SystemTime>='{}']]]",
        iso_utc(since_ms)
    );
    // SAFETY: a forward query over one channel; closed below.
    let results = unsafe {
        EvtQuery(
            None,
            &HSTRING::from("Application"),
            &HSTRING::from(query),
            EvtQueryChannelPath.0 | EvtQueryForwardDirection.0,
        )?
    };
    let mut out = Vec::new();
    loop {
        let mut batch = [0isize; 16];
        let mut returned = 0u32;
        // SAFETY: `batch` receives up to 16 event handles, each closed below.
        if unsafe { EvtNext(results, &mut batch, 2000, 0, &mut returned) }.is_err() {
            break;
        }
        for raw in &batch[..(returned as usize).min(batch.len())] {
            let event = EVT_HANDLE(*raw);
            if let Some(found) =
                render_text(event, EvtRenderEventXml).and_then(|x| restore_event(&x))
            {
                out.push(found);
            }
            // SAFETY: closes the event handle.
            let _ = unsafe { EvtClose(event) };
        }
    }
    // SAFETY: closes the query handle.
    let _ = unsafe { EvtClose(results) };
    Ok(out)
}

/// Reads a System Restore event's XML: its id, its time and the description, the second data field.
pub fn restore_event(xml: &str) -> Option<RestoreEvent> {
    let between = |open: &str, close: &str| -> Option<&str> {
        let start = xml.find(open)? + open.len();
        let len = xml[start..].find(close)?;
        Some(&xml[start..start + len])
    };
    let id = between("<EventID", "</EventID>")?
        .rsplit('>')
        .next()?
        .trim()
        .parse()
        .ok()?;
    let time = between("SystemTime='", "'").or_else(|| between("SystemTime=\"", "\""))?;
    let description = xml.split("<Data>").nth(2)?.split("</Data>").next()?;
    Some(RestoreEvent {
        ts_ms: unix_ms(time)?,
        id,
        description: description.to_string(),
    })
}

/// Unix milliseconds of an event's UTC time such as "2026-11-03T00:40:03.3094377Z".
fn unix_ms(time: &str) -> Option<i64> {
    let (date, clock) = time.trim_end_matches('Z').split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>());
    let (y, m, day) = (d.next()?.ok()?, d.next()?.ok()?, d.next()?.ok()?);
    let mut c = clock.split(':');
    let (h, min) = (
        c.next()?.parse::<i64>().ok()?,
        c.next()?.parse::<i64>().ok()?,
    );
    let secs: f64 = c.next()?.parse().ok()?;
    // Days from 1970 for a proleptic Gregorian date.
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + day - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    Some(((days * 24 + h) * 60 + min) * 60_000 + (secs * 1000.0) as i64)
}

/// An event query's UTC time for Unix milliseconds.
fn iso_utc(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.000Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The Win32 error behind a failed call.
fn code(err: &windows::core::Error) -> u32 {
    WIN32_ERROR::from_error(err).map_or(err.code().0 as u32, |e| e.0)
}

fn ok(status: WIN32_ERROR) -> Result<(), u32> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(status.0)
    }
}

/// Writes one setting; elevation is needed for every hive but the user's and for services.
pub fn write_setting(setting: &Setting, value: &Value) -> Result<(), u32> {
    match (setting, value) {
        (Setting::Dword { hive, key, name }, Value::Dword(v)) => {
            // SAFETY: writes one DWORD from a 4-byte buffer; creates the key when missing.
            ok(unsafe {
                RegSetKeyValueW(
                    root(*hive),
                    &HSTRING::from(key.as_str()),
                    &HSTRING::from(name.as_str()),
                    REG_DWORD.0,
                    Some((&raw const *v).cast()),
                    4,
                )
            })
        }
        (Setting::Dword { hive, key, name }, Value::Absent) => delete_value(root(*hive), key, name),
        (Setting::StartupApproval { hive, list, name }, Value::Absent) => {
            delete_value(root(*hive), list.approval_key(), name)
        }
        (Setting::StartupApproval { hive, list, name }, Value::Bytes(bytes)) => {
            // SAFETY: writes a binary value from a buffer of the length passed.
            ok(unsafe {
                RegSetKeyValueW(
                    root(*hive),
                    &HSTRING::from(list.approval_key()),
                    &HSTRING::from(name.as_str()),
                    REG_BINARY.0,
                    Some(bytes.as_ptr().cast()),
                    bytes.len() as u32,
                )
            })
        }
        (Setting::Key { hive, key }, Value::Present) => {
            let mut created = HKEY::default();
            // SAFETY: creates or opens a key for writing; closed right after.
            let status = unsafe {
                RegCreateKeyExW(
                    root(*hive),
                    &HSTRING::from(key.as_str()),
                    None,
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_WRITE,
                    None,
                    &mut created,
                    None,
                )
            };
            if status == ERROR_SUCCESS {
                // SAFETY: closes the key opened above.
                let _ = unsafe { RegCloseKey(created) };
            }
            ok(status)
        }
        (Setting::Key { hive, key }, Value::Absent) => delete_empty_key(root(*hive), key),
        (
            Setting::Service { name },
            Value::Service {
                start,
                delayed,
                running,
            },
        ) => write_service(name, *start, *delayed, *running),
        (
            Setting::PowerAc {
                subgroup,
                setting,
                scheme,
            },
            Value::Index(index),
        ) => {
            let chosen = scheme_of(*scheme).ok_or(ERROR_FILE_NOT_FOUND.0)?;
            let Some(Value::Index(before)) = read_setting(&Setting::PowerAc {
                subgroup: *subgroup,
                setting: *setting,
                scheme: Some(chosen),
            }) else {
                return Err(ERROR_FILE_NOT_FOUND.0);
            };
            write_ac_index(chosen, *subgroup, *setting, *index)?;
            if active_scheme() == Some(chosen) {
                // SAFETY: applies the active scheme again so the new value takes effect.
                let applied =
                    ok(unsafe { PowerSetActiveScheme(None, Some(&GUID::from_u128(chosen.0))) });
                if let Err(first) = applied {
                    let back = write_ac_index(chosen, *subgroup, *setting, before);
                    return Err(after_rollback(first, back));
                }
            }
            Ok(())
        }
        (Setting::ActiveScheme, Value::Scheme(scheme)) => {
            // SAFETY: makes an existing scheme the active one.
            ok(unsafe { PowerSetActiveScheme(None, Some(&GUID::from_u128(scheme.0))) })
        }
        (Setting::Hibernation, Value::On(on)) => {
            enable_privilege("SeCreatePagefilePrivilege")?;
            let wanted = u8::from(*on);
            // SAFETY: the input buffer is the one-byte BOOLEAN the information level reads.
            let status = unsafe {
                CallNtPowerInformation(
                    SystemReserveHiberFile,
                    Some((&raw const wanted).cast()),
                    1,
                    None,
                    0,
                )
            };
            if status == STATUS_SUCCESS {
                Ok(())
            } else {
                Err(status.0 as u32)
            }
        }
        _ => Err(ERROR_FILE_NOT_FOUND.0),
    }
}

/// Deletes one value; a value already gone counts as deleted.
fn delete_value(hive: HKEY, key: &str, name: &str) -> Result<(), u32> {
    // SAFETY: deletes one value of one key; both names are owned strings.
    let status = unsafe { RegDeleteKeyValueW(hive, &HSTRING::from(key), &HSTRING::from(name)) };
    if status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        ok(status)
    }
}

/// Deletes a key that holds no value and no subkey; one that does stays.
fn delete_empty_key(hive: HKEY, key: &str) -> Result<(), u32> {
    let mut opened = HKEY::default();
    // SAFETY: opens a key for reading; closed below.
    let status = unsafe { RegOpenKeyExW(hive, &HSTRING::from(key), None, KEY_READ, &mut opened) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    ok(status)?;
    let (mut subkeys, mut values) = (0u32, 0u32);
    // SAFETY: asks only for the two counts.
    let status = unsafe {
        RegQueryInfoKeyW(
            opened,
            None,
            None,
            None,
            Some(&mut subkeys),
            None,
            None,
            Some(&mut values),
            None,
            None,
            None,
            None,
        )
    };
    // SAFETY: closes the key opened above.
    let _ = unsafe { RegCloseKey(opened) };
    ok(status)?;
    if subkeys + values > 0 {
        return Err(ERROR_DIR_NOT_EMPTY.0);
    }
    // SAFETY: deletes the empty key.
    ok(unsafe { RegDeleteKeyW(hive, &HSTRING::from(key)) })
}

/// The calls `write_service` makes on one open service.
trait ServiceCalls {
    fn set_start(&mut self, start: u32) -> Result<(), u32>;
    fn set_delayed(&mut self, delayed: bool) -> Result<(), u32>;
    fn is_running(&mut self) -> Result<bool, u32>;
    fn stop(&mut self) -> Result<(), u32>;
    fn start_it(&mut self) -> Result<(), u32>;
}

/// Moves a service to `wanted` (start type, delayed start, running) from `before` (start type,
/// delayed start). When a step after the start type fails, the old start type is written back and
/// the first error returned; if that fails too, the error says the value may have changed.
fn switch_service(
    calls: &mut impl ServiceCalls,
    before: (u32, bool),
    wanted: (u32, bool, bool),
) -> Result<(), u32> {
    let (start, delayed, running) = wanted;
    calls.set_start(start)?;
    let later = (|| {
        calls.set_delayed(start == 2 && delayed)?;
        let is_running = calls.is_running()?;
        if is_running && !running {
            calls.stop()
        } else if !is_running && running {
            calls.start_it()
        } else {
            Ok(())
        }
    })();
    later.map_err(|first| {
        let back = calls
            .set_start(before.0)
            .and_then(|()| calls.set_delayed(before.0 == 2 && before.1));
        after_rollback(first, back)
    })
}

/// The error to report once a rollback ran: the first one, unless the rollback failed as well.
fn after_rollback(first: u32, rollback: Result<(), u32>) -> u32 {
    if rollback.is_ok() {
        first
    } else {
        ERROR_MAY_HAVE_CHANGED
    }
}

/// One service opened to change and control it, closed when dropped.
struct OpenService(Service);

impl ServiceCalls for OpenService {
    fn set_start(&mut self, start: u32) -> Result<(), u32> {
        // SAFETY: changes only the start type; every other field is left as it is.
        unsafe {
            ChangeServiceConfigW(
                (self.0).0,
                ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE),
                SERVICE_START_TYPE(start),
                SERVICE_ERROR(SERVICE_NO_CHANGE),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
        .map_err(|e| code(&e))
    }

    fn set_delayed(&mut self, delayed: bool) -> Result<(), u32> {
        let mut info = SERVICE_DELAYED_AUTO_START_INFO {
            fDelayedAutostart: delayed.into(),
        };
        // SAFETY: the struct is the one the information level reads.
        unsafe {
            ChangeServiceConfig2W(
                (self.0).0,
                SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
                Some((&raw mut info).cast()),
            )
        }
        .map_err(|e| code(&e))
    }

    fn is_running(&mut self) -> Result<bool, u32> {
        let mut status = SERVICE_STATUS::default();
        // SAFETY: the handle has SERVICE_QUERY_STATUS.
        unsafe { QueryServiceStatus((self.0).0, &mut status) }.map_err(|e| code(&e))?;
        Ok(status.dwCurrentState == SERVICE_RUNNING)
    }

    fn stop(&mut self) -> Result<(), u32> {
        let mut status = SERVICE_STATUS::default();
        // SAFETY: the handle has SERVICE_STOP; the status buffer is the one the call fills.
        unsafe { ControlService((self.0).0, SERVICE_CONTROL_STOP, &mut status) }
            .map_err(|e| code(&e))
    }

    fn start_it(&mut self) -> Result<(), u32> {
        // SAFETY: the handle has SERVICE_START; no arguments are passed.
        unsafe { StartServiceW((self.0).0, None) }.map_err(|e| code(&e))
    }
}

fn write_service(name: &str, start: u32, delayed: bool, running: bool) -> Result<(), u32> {
    let Some(Value::Service {
        start: old_start,
        delayed: old_delayed,
        ..
    }) = read_service(name)
    else {
        return Err(ERROR_FILE_NOT_FOUND.0);
    };
    // SAFETY: a connection to the local service manager; closed by `Service`.
    let manager =
        Service(unsafe { OpenSCManagerW(None, None, SC_MANAGER_CONNECT) }.map_err(|e| code(&e))?);
    let access = SERVICE_CHANGE_CONFIG | SERVICE_QUERY_STATUS | SERVICE_START | SERVICE_STOP;
    // SAFETY: opens one service to change it; closed by `Service`.
    let service = Service(
        unsafe { OpenServiceW(manager.0, &HSTRING::from(name), access) }.map_err(|e| code(&e))?,
    );
    switch_service(
        &mut OpenService(service),
        (old_start, old_delayed),
        (start, delayed, running),
    )
}

/// Turns on one privilege the process token holds but keeps off by default.
fn enable_privilege(name: &str) -> Result<(), u32> {
    let mut token = HANDLE::default();
    // SAFETY: opens this process's own token; closed below.
    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
    }
    .map_err(|e| code(&e))?;
    let mut luid = LUID::default();
    // SAFETY: looks the privilege name up on this machine.
    let looked = unsafe { LookupPrivilegeValueW(None, &HSTRING::from(name), &mut luid) };
    let result = looked.map_err(|e| code(&e)).and_then(|()| {
        let mut privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            ..Default::default()
        };
        privileges.Privileges[0].Luid = luid;
        privileges.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        // SAFETY: one privilege in a struct sized for one; the previous state is not asked for.
        unsafe { AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None) }
            .map_err(|e| code(&e))
    });
    // SAFETY: closes the token opened above.
    let _ = unsafe { CloseHandle(token) };
    result
}

/// The settings of this PC, read and written through the Windows API.
pub struct WindowsSettings;

impl SettingStore for WindowsSettings {
    fn read(&mut self, setting: &Setting) -> Option<Value> {
        read_setting(setting)
    }

    fn write(&mut self, setting: &Setting, value: &Value) -> Result<(), u32> {
        write_setting(setting, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A service in memory that records its start type and can refuse a call.
    struct FakeService {
        start: u32,
        delayed: bool,
        running: bool,
        refuses_stop: bool,
        refuses_rollback: bool,
        calls: usize,
    }

    impl ServiceCalls for FakeService {
        fn set_start(&mut self, start: u32) -> Result<(), u32> {
            self.calls += 1;
            if self.refuses_rollback && self.calls > 1 {
                return Err(1053);
            }
            self.start = start;
            Ok(())
        }

        fn set_delayed(&mut self, delayed: bool) -> Result<(), u32> {
            self.delayed = delayed;
            Ok(())
        }

        fn is_running(&mut self) -> Result<bool, u32> {
            Ok(self.running)
        }

        fn stop(&mut self) -> Result<(), u32> {
            if self.refuses_stop {
                return Err(1052);
            }
            self.running = false;
            Ok(())
        }

        fn start_it(&mut self) -> Result<(), u32> {
            self.running = true;
            Ok(())
        }
    }

    fn running_automatic() -> FakeService {
        FakeService {
            start: 2,
            delayed: true,
            running: true,
            refuses_stop: false,
            refuses_rollback: false,
            calls: 0,
        }
    }

    #[test]
    fn a_service_that_refuses_stop_keeps_its_start_type() {
        let mut service = FakeService {
            refuses_stop: true,
            ..running_automatic()
        };
        let result = switch_service(&mut service, (2, true), (4, false, false));
        assert_eq!(result, Err(1052));
        assert_eq!((service.start, service.delayed), (2, true));
        assert!(service.running);
    }

    #[test]
    fn a_service_that_stops_is_left_disabled() {
        let mut service = running_automatic();
        assert_eq!(
            switch_service(&mut service, (2, true), (4, false, false)),
            Ok(())
        );
        assert_eq!(
            (service.start, service.delayed, service.running),
            (4, false, false)
        );
    }

    #[test]
    fn a_failed_rollback_says_the_value_may_have_changed() {
        let mut service = FakeService {
            refuses_stop: true,
            refuses_rollback: true,
            ..running_automatic()
        };
        let result = switch_service(&mut service, (2, true), (4, false, false));
        assert_eq!(result, Err(ERROR_MAY_HAVE_CHANGED));
    }

    #[test]
    fn a_rollback_never_hides_the_first_error() {
        assert_eq!(after_rollback(1052, Ok(())), 1052);
        assert_eq!(after_rollback(1052, Err(5)), ERROR_MAY_HAVE_CHANGED);
    }
}
