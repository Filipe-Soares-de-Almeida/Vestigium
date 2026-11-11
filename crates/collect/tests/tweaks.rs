use vestigium_collect::{RESTORE_SKIPPED, WindowsSettings, exe_of, read_setting, restore_event};
use vestigium_core::{
    Hive, PCIE_ASPM, PlanItem, SCHEME_BALANCED, SCHEME_POWER_SAVER, SUB_PCIEXPRESS, Setting,
    SettingChange, StartupList, TweakId, Value, carry_out, undo,
};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, RegDeleteTreeW, RegSetKeyValueW,
};
use windows::core::HSTRING;

/// A key of its own under HKCU, deleted with everything in it when dropped.
struct TestKey(String);

impl TestKey {
    fn new(test: &str) -> TestKey {
        TestKey(format!(
            r"Software\Vestigium-test-{}-{test}",
            std::process::id()
        ))
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
    let key = TestKey::new("read");
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
    let key = TestKey::new("write");
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

/// Changes the power schemes of the machine: run it by hand on a disposable virtual machine with
/// `cargo test -p vestigium-collect --test tweaks -- --ignored power_undo_goes_to_the_scheme_changed`.
#[test]
#[ignore = "changes the active power scheme; run on a disposable virtual machine"]
fn power_undo_goes_to_the_scheme_changed() {
    let aspm = |scheme| Setting::PowerAc {
        subgroup: SUB_PCIEXPRESS,
        setting: PCIE_ASPM,
        scheme: Some(scheme),
    };
    let mut store = WindowsSettings;
    let Some(Value::Scheme(original)) = read_setting(&Setting::ActiveScheme) else {
        panic!("the active scheme could not be read");
    };
    let item = |setting, to, undo_of| PlanItem {
        tweak: TweakId::PcieAspm,
        subject: None,
        undo_of,
        undo_journal: None,
        changes: vec![SettingChange {
            setting,
            from: Value::Absent,
            to,
        }],
    };
    let set_active = |scheme| {
        let plan = PlanItem {
            tweak: TweakId::PowerPlan,
            subject: None,
            undo_of: None,
            undo_journal: None,
            changes: vec![SettingChange {
                setting: Setting::ActiveScheme,
                from: Value::Absent,
                to: Value::Scheme(scheme),
            }],
        };
        assert!(carry_out(&plan, &mut WindowsSettings).succeeded());
    };
    set_active(SCHEME_POWER_SAVER);
    let saver_before = read_setting(&aspm(SCHEME_POWER_SAVER));
    let balanced_before = read_setting(&aspm(SCHEME_BALANCED));

    let applied = carry_out(
        &item(
            Setting::PowerAc {
                subgroup: SUB_PCIEXPRESS,
                setting: PCIE_ASPM,
                scheme: None,
            },
            Value::Index(0),
            None,
        ),
        &mut store,
    );
    assert!(applied.succeeded());
    set_active(SCHEME_BALANCED);
    let written: Vec<_> = applied.results.iter().map(|r| r.change.clone()).collect();
    let back = PlanItem {
        changes: undo(&written),
        ..item(aspm(SCHEME_POWER_SAVER), Value::Absent, Some(1))
    };
    let undone = carry_out(&back, &mut store);

    assert!(undone.succeeded());
    assert_eq!(read_setting(&aspm(SCHEME_POWER_SAVER)), saver_before);
    assert_eq!(read_setting(&aspm(SCHEME_BALANCED)), balanced_before);
    set_active(original);
}
