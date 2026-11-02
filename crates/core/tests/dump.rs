use vestigium_core::{
    Analysis, Confidence, DumpCulprit, DumpKind, DumpRecord, Fault, bug_check_report, dump_culprit,
    dump_of_fall, fault_address, kernel_dump_kind,
};

const MINUTE: i64 = 60_000;

fn blue_screen(code: u32, params: [u64; 4], fault: Option<&str>, stack: &[&str]) -> Analysis {
    Analysis {
        fault: Fault::BugCheck { code, params },
        fault_module: fault.map(str::to_string),
        stack_modules: stack.iter().map(|m| m.to_string()).collect(),
        process: None,
        module_count: 180,
    }
}

fn app_crash(fault: Option<&str>, stack: &[&str]) -> Analysis {
    Analysis {
        fault: Fault::Exception {
            code: 0xC000_0005,
            address: 0x7FF6_0000_1234,
        },
        fault_module: fault.map(str::to_string),
        stack_modules: stack.iter().map(|m| m.to_string()).collect(),
        process: Some("Editor.exe".into()),
        module_count: 90,
    }
}

fn named(culprit: Option<DumpCulprit>) -> Option<(String, Confidence)> {
    match culprit? {
        DumpCulprit::Module {
            name, confidence, ..
        } => Some((name, confidence)),
        _ => None,
    }
}

fn record(kind: DumpKind, crash_ms: i64, code: Option<u32>) -> DumpRecord {
    DumpRecord {
        path: format!(r"C:\Windows\{kind:?}.dmp"),
        kind,
        crash_ms,
        size_bytes: 1 << 20,
        analysis: code.map(|c| blue_screen(c, [0; 4], None, &[])),
    }
}

#[test]
fn the_fault_address_comes_from_the_parameter_the_stop_code_names() {
    let irql = Fault::BugCheck {
        code: 0x0A,
        params: [0x10, 2, 0, 0xFFFF_F801_0000_4000],
    };
    assert_eq!(fault_address(&irql), Some(0xFFFF_F801_0000_4000));
    let page = Fault::BugCheck {
        code: 0x50,
        params: [0x10, 0, 0xFFFF_F801_0000_5000, 2],
    };
    assert_eq!(fault_address(&page), Some(0xFFFF_F801_0000_5000));
    let watchdog = Fault::BugCheck {
        code: 0x133,
        params: [1, 0x1E00, 0, 0],
    };
    assert_eq!(fault_address(&watchdog), None);
}

#[test]
fn kernel_culprit_skips_the_kernel() {
    let in_driver = blue_screen(0xD1, [0; 4], Some("netdrv.sys"), &["ntkrnlmp.exe"]);
    assert_eq!(
        named(dump_culprit(&in_driver)),
        Some(("netdrv.sys".into(), Confidence::High))
    );

    let in_kernel = blue_screen(
        0x0A,
        [0; 4],
        Some("ntkrnlmp.exe"),
        &["ntkrnlmp.exe", "hal.dll"],
    );
    assert_eq!(
        dump_culprit(&in_kernel),
        None,
        "only the kernel is no culprit"
    );

    let on_stack = blue_screen(
        0x0A,
        [0; 4],
        Some("ntoskrnl.exe"),
        &["ntoskrnl.exe", "audio.sys"],
    );
    assert_eq!(
        named(dump_culprit(&on_stack)),
        Some(("audio.sys".into(), Confidence::Medium))
    );
}

#[test]
fn intermediary_lowers_confidence() {
    let behind = blue_screen(0x0A, [0; 4], Some("ndis.sys"), &["ndis.sys", "wifi.sys"]);
    assert_eq!(
        named(dump_culprit(&behind)),
        Some(("wifi.sys".into(), Confidence::Medium)),
        "the driver behind the framework is named"
    );
    let alone = blue_screen(0x0A, [0; 4], Some("storport.sys"), &["ntoskrnl.exe"]);
    assert!(matches!(
        dump_culprit(&alone),
        Some(DumpCulprit::Module {
            confidence: Confidence::Low,
            intermediary: true,
            ..
        })
    ));
}

#[test]
fn a_hardware_stop_blames_the_hardware() {
    let whea = blue_screen(0x124, [0; 4], Some("ntoskrnl.exe"), &[]);
    assert_eq!(dump_culprit(&whea), Some(DumpCulprit::Hardware));
}

#[test]
fn app_crash_in_a_system_dll_blames_the_app() {
    let own = app_crash(Some("Plugin.dll"), &["Plugin.dll", "ntdll.dll"]);
    assert_eq!(
        named(dump_culprit(&own)),
        Some(("Plugin.dll".into(), Confidence::High))
    );
    let below = app_crash(
        Some("ntdll.dll"),
        &["ntdll.dll", "KERNELBASE.dll", "Engine.dll"],
    );
    assert_eq!(
        named(dump_culprit(&below)),
        Some(("Engine.dll".into(), Confidence::Medium))
    );
    let only_system = app_crash(Some("ucrtbase.dll"), &["ucrtbase.dll", "ntdll.dll"]);
    assert_eq!(
        dump_culprit(&only_system),
        Some(DumpCulprit::App {
            name: "Editor.exe".into()
        })
    );
}

#[test]
fn dump_links_to_its_postmortem() {
    let down = 1_000 * MINUTE;
    let dumps = [
        record(DumpKind::KernelMemory, down + MINUTE, Some(0x50)),
        record(DumpKind::KernelMini, down + MINUTE, Some(0x50)),
        record(DumpKind::KernelMini, down + 30 * MINUTE, Some(0x50)),
        record(DumpKind::KernelMini, down, Some(0x0A)),
    ];
    let found = dump_of_fall(&dumps, down, 0x50).unwrap();
    assert_eq!(
        found.kind,
        DumpKind::KernelMini,
        "the minidump of the fall comes first"
    );
    assert_eq!(found.crash_ms, down + MINUTE);
    assert!(dump_of_fall(&dumps, down + 10 * MINUTE, 0x50).is_none());

    let unread = [record(DumpKind::KernelMemory, down + 2 * MINUTE, None)];
    assert!(
        dump_of_fall(&unread, down, 0x50).is_some(),
        "an unread dump may still be the one"
    );
}

#[test]
fn bug_check_text_gives_code_and_params() {
    let text = "0x0000000a (0xffffa0001234beef, 0x0000000000000002, 0x0000000000000000, \
                0xfffff80012345678)";
    assert_eq!(
        bug_check_report(text),
        Some((0x0A, [0xFFFF_A000_1234_BEEF, 2, 0, 0xFFFF_F800_1234_5678]))
    );
    assert_eq!(bug_check_report("0x00000050"), None);
    assert_eq!(bug_check_report("0x00000050 (0x1, 0x2, 0x3)"), None);
    assert_eq!(bug_check_report("0xZZ (0x1, 0x2, 0x3, 0x4)"), None);
}

#[test]
fn the_folder_says_the_kind_of_a_kernel_dump() {
    assert_eq!(
        kernel_dump_kind(r"C:\Windows\Minidump\110226-1-01.dmp"),
        DumpKind::KernelMini
    );
    assert_eq!(
        kernel_dump_kind(r"C:\Windows\LiveKernelReports\WATCHDOG\WD-1.dmp"),
        DumpKind::LiveKernel
    );
    assert_eq!(
        kernel_dump_kind(r"C:\Windows\MEMORY.DMP"),
        DumpKind::KernelMemory
    );
}
