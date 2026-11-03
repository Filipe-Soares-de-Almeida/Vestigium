use vestigium_collect::{RESTORE_SKIPPED, exe_of, read_setting, restore_event};
use vestigium_core::{Hive, Setting, StartupList, Value};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, RegDeleteTreeW, RegSetKeyValueW,
};
use windows::core::HSTRING;

/// A key of its own under HKCU, deleted with everything in it when dropped.
struct TestKey(String);

impl TestKey {
    fn new() -> TestKey {
        TestKey(format!(r"Software\Vestigium-test-{}", std::process::id()))
    }

    fn set_dword(&self, name: &str, value: u32) {
        // SAFETY: writes one DWORD from a 4-byte buffer; creates the key when missing.
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &HSTRING::from(self.0.as_str()),
                &HSTRING::from(name),
                REG_DWORD.0,
                Some((&raw const value).cast()),
                4,
            )
        };
        assert_eq!(status, ERROR_SUCCESS);
    }
}

impl Drop for TestKey {
    fn drop(&mut self) {
        // SAFETY: deletes only the test's own key.
        let _ = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(self.0.as_str())) };
    }
}

#[test]
fn reads_a_dword_and_an_absent_value() {
    let key = TestKey::new();
    let dword = |name: &str| Setting::Dword {
        hive: Hive::CurrentUser,
        key: key.0.clone(),
        name: name.into(),
    };
    let as_key = Setting::Key {
        hive: Hive::CurrentUser,
        key: key.0.clone(),
    };
    assert_eq!(read_setting(&as_key), Some(Value::Absent));
    assert_eq!(read_setting(&dword("Answer")), Some(Value::Absent));
    key.set_dword("Answer", 42);
    assert_eq!(read_setting(&dword("Answer")), Some(Value::Dword(42)));
    assert_eq!(read_setting(&dword("Missing")), Some(Value::Absent));
    assert_eq!(read_setting(&as_key), Some(Value::Present));
}

#[test]
fn a_missing_approval_reads_as_absent() {
    let approval = Setting::StartupApproval {
        hive: Hive::CurrentUser,
        list: StartupList::Run,
        name: format!("Vestigium-test-{}", std::process::id()),
    };
    assert_eq!(read_setting(&approval), Some(Value::Absent));
}

#[test]
fn a_missing_service_cannot_be_read() {
    let service = Setting::Service {
        name: "VestigiumNoSuchService".into(),
    };
    assert_eq!(read_setting(&service), None);
}

#[test]
fn exe_of_command_lines() {
    assert_eq!(
        exe_of(r#""C:\Program Files\Chat\Chat.exe" --minimized"#),
        "Chat.exe"
    );
    assert_eq!(
        exe_of(r"C:\Program Files\Notes\notes.exe -silent"),
        "notes.exe"
    );
    assert_eq!(exe_of(r"%ProgramFiles%\Tray\Tray.EXE"), "Tray.EXE");
    assert_eq!(
        exe_of(r"rundll32.exe C:\Tools\helper.dll,Start"),
        "rundll32.exe"
    );
    assert_eq!(exe_of("launcher --quiet"), "launcher");
}

#[test]
fn restore_event_reads_the_description() {
    let xml = "<Event><System><Provider Name='System Restore'/>\
        <EventID Qualifiers='0'>8216</EventID>\
        <TimeCreated SystemTime='2026-11-03T00:40:03.3094377Z'/></System>\
        <EventData><Data>C:\\Tools\\helper.exe tweak</Data><Data>Vestigium 1793666400000</Data>\
        <Binary>00</Binary></EventData></Event>";
    let event = restore_event(xml).expect("parsed");
    assert_eq!(event.id, RESTORE_SKIPPED);
    assert_eq!(event.ts_ms, 1_793_666_403_309);
    assert_eq!(event.description, "Vestigium 1793666400000");
}

#[test]
fn the_restore_query_is_accepted() {
    let since = vestigium_collect::wall_ms() - 365 * 24 * 60 * 60_000;
    let events = vestigium_collect::restore_events(since).expect("query");
    assert!(events.iter().all(|e| e.ts_ms >= since));
}

#[test]
fn writes_and_deletes_a_dword_and_an_empty_key() {
    let key = TestKey::new();
    let answer = Setting::Dword {
        hive: Hive::CurrentUser,
        key: key.0.clone(),
        name: "Answer".into(),
    };
    vestigium_collect::write_setting(&answer, &Value::Dword(7)).unwrap();
    assert_eq!(read_setting(&answer), Some(Value::Dword(7)));
    vestigium_collect::write_setting(&answer, &Value::Absent).unwrap();
    assert_eq!(read_setting(&answer), Some(Value::Absent));
    vestigium_collect::write_setting(&answer, &Value::Absent).unwrap();

    let sub = Setting::Key {
        hive: Hive::CurrentUser,
        key: format!(r"{}\Sub", key.0),
    };
    vestigium_collect::write_setting(&sub, &Value::Present).unwrap();
    assert_eq!(read_setting(&sub), Some(Value::Present));
    let inside = Setting::Dword {
        hive: Hive::CurrentUser,
        key: format!(r"{}\Sub", key.0),
        name: "Kept".into(),
    };
    vestigium_collect::write_setting(&inside, &Value::Dword(1)).unwrap();
    assert!(vestigium_collect::write_setting(&sub, &Value::Absent).is_err());
    vestigium_collect::write_setting(&inside, &Value::Absent).unwrap();
    vestigium_collect::write_setting(&sub, &Value::Absent).unwrap();
    assert_eq!(read_setting(&sub), Some(Value::Absent));
}
