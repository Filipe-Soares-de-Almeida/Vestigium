use std::path::Path;

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_ROUTINE_FLAGS, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ,
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::core::HSTRING;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const VALUE: &str = "Vestigium";

/// True when Windows will start this executable at logon.
pub fn is_enabled(exe: &Path) -> bool {
    let registered = read(RUN_KEY, RRF_RT_REG_SZ)
        .map(|bytes| utf16_string(&bytes))
        .is_some_and(|value| run_value_matches(&value, exe));
    registered && startup_approved_allows(read(APPROVED_KEY, RRF_RT_REG_BINARY).as_deref())
}

/// Registers `exe` to start at logon, clearing a "disabled" mark left by Task Manager.
pub fn enable(exe: &Path) -> windows::core::Result<()> {
    let wide: Vec<u16> = format!("\"{}\"", exe.display())
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: `wide` is a NUL-terminated UTF-16 string of the byte length passed.
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY),
            &HSTRING::from(VALUE),
            REG_SZ.0,
            Some(wide.as_ptr().cast()),
            (wide.len() * 2) as u32,
        )
        .ok()?
    };
    delete(APPROVED_KEY)
}

pub fn disable() -> windows::core::Result<()> {
    delete(RUN_KEY)
}

fn delete(subkey: &str) -> windows::core::Result<()> {
    // SAFETY: deletes one named value under HKCU; both strings outlive the call.
    let status = unsafe {
        RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(subkey),
            &HSTRING::from(VALUE),
        )
    };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        status.ok()
    }
}

fn read(subkey: &str, flags: REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
    let (subkey, value) = (HSTRING::from(subkey), HSTRING::from(VALUE));
    let mut size = 0u32;
    // SAFETY: a size query with no buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &value,
            flags,
            None,
            None,
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    // SAFETY: `data` has `size` writable bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &value,
            flags,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    data.truncate(size as usize);
    (status == ERROR_SUCCESS).then_some(data)
}

fn utf16_string(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .take_while(|&unit| unit != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// Compares a Run value, quoted or not, with `exe`, ignoring case.
fn run_value_matches(value: &str, exe: &Path) -> bool {
    let value = value.trim();
    let path = match value.strip_prefix('"') {
        Some(rest) => rest.split('"').next().unwrap_or(rest),
        None => value,
    };
    path.to_lowercase() == exe.to_string_lossy().to_lowercase()
}

/// Task Manager marks a disabled entry by setting bit 0 of the first byte.
fn startup_approved_allows(data: Option<&[u8]>) -> bool {
    data.and_then(|bytes| bytes.first())
        .is_none_or(|flags| flags & 1 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Apps\Vestigium\vestigium-agent.exe";

    #[test]
    fn run_value_matches_quoted_path() {
        let value = r#""C:\apps\vestigium\VESTIGIUM-AGENT.exe""#;
        assert!(run_value_matches(value, Path::new(EXE)));
    }

    #[test]
    fn run_value_matches_unquoted_path() {
        assert!(run_value_matches(EXE, Path::new(EXE)));
    }

    #[test]
    fn run_value_rejects_another_path() {
        let value = r#""C:\Old\Vestigium\vestigium-agent.exe""#;
        assert!(!run_value_matches(value, Path::new(EXE)));
    }

    #[test]
    fn startup_approved_absent_allows() {
        assert!(startup_approved_allows(None));
        assert!(startup_approved_allows(Some(&[])));
    }

    #[test]
    fn startup_approved_enabled_values_allow() {
        assert!(startup_approved_allows(Some(&[2, 0, 0, 0, 0, 0, 0, 0])));
        assert!(startup_approved_allows(Some(&[6, 0, 0, 0, 0, 0, 0, 0])));
    }

    #[test]
    fn startup_approved_disabled_by_task_manager() {
        assert!(!startup_approved_allows(Some(&[3, 0, 0, 0, 9, 9, 9, 9])));
        assert!(!startup_approved_allows(Some(&[7, 0, 0, 0, 9, 9, 9, 9])));
    }

    #[test]
    fn utf16_string_stops_at_nul() {
        let bytes: Vec<u8> = "ab\0junk"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(utf16_string(&bytes), "ab");
    }
}
