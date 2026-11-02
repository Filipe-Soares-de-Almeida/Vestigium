//! Opens a crash dump with the debugger engine that ships with Windows, with no symbols and no
//! network, and prints what it says: `dump_probe <file.dmp>`.

use std::ffi::c_void;
use std::time::Instant;

use vestigium_collect::ProcessSnapshotter;
use windows::Win32::System::Diagnostics::Debug::Extensions::{
    DEBUG_EVENT_EXCEPTION, DEBUG_LAST_EVENT_INFO_EXCEPTION, DEBUG_MODNAME_MODULE,
    DEBUG_MODULE_PARAMETERS, DEBUG_STACK_FRAME, DEBUG_WAIT_DEFAULT, DebugCreate, IDebugClient4,
    IDebugControl4, IDebugDataSpaces, IDebugRegisters, IDebugSymbols3,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::{HSTRING, Interface, s};

/// Bytes of stack read from the stack pointer of the faulting context.
const STACK_BYTES: usize = 0x3000;
const FRAMES: usize = 32;

struct Module {
    name: String,
    base: u64,
    size: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: dump_probe <file.dmp>")?;
    // SAFETY: called before any other thread exists in this process.
    unsafe {
        std::env::remove_var("_NT_SYMBOL_PATH");
        std::env::remove_var("_NT_ALT_SYMBOL_PATH");
    }
    let mut own = ProcessSnapshotter::with_capacity(1 << 20);
    let private_now = |s: &mut ProcessSnapshotter| {
        s.snapshot()
            .ok()
            .and_then(|all| all.into_iter().find(|p| p.pid == std::process::id()))
            .map(|p| (p.working_set, p.private_bytes))
            .unwrap_or_default()
    };
    let before = private_now(&mut own);
    let started = Instant::now();

    // SAFETY: plain engine calls on interfaces created here; buffers outlive every call.
    unsafe {
        let client: IDebugClient4 = DebugCreate()?;
        let control: IDebugControl4 = client.cast()?;
        let symbols: IDebugSymbols3 = client.cast()?;
        let data: IDebugDataSpaces = client.cast()?;
        let registers: IDebugRegisters = client.cast()?;
        symbols.SetSymbolPath(s!(""))?;
        symbols.SetImagePath(s!(""))?;
        client.OpenDumpFileWide(&HSTRING::from(path.as_str()), 0)?;
        control.WaitForEvent(DEBUG_WAIT_DEFAULT, u32::MAX)?;
        println!(
            "opened in {:.0} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        println!(
            "format flags: {:#x}",
            control.GetDumpFormatFlags().unwrap_or(0)
        );

        let (mut code, mut a1, mut a2, mut a3, mut a4) = (0u32, 0u64, 0u64, 0u64, 0u64);
        match control.ReadBugCheckData(&mut code, &mut a1, &mut a2, &mut a3, &mut a4) {
            Ok(()) => println!("bug check {code:#010x} ({a1:#x}, {a2:#x}, {a3:#x}, {a4:#x})"),
            Err(err) => println!("bug check: none ({err})"),
        }

        let (mut kind, mut pid, mut tid) = (0u32, 0u32, 0u32);
        let mut exception = DEBUG_LAST_EVENT_INFO_EXCEPTION::default();
        let mut description = [0u8; 256];
        let event = control.GetLastEventInformation(
            &mut kind,
            &mut pid,
            &mut tid,
            Some(&raw mut exception as *mut c_void),
            size_of::<DEBUG_LAST_EVENT_INFO_EXCEPTION>() as u32,
            None,
            Some(&mut description),
            None,
        );
        let text = String::from_utf8_lossy(&description);
        println!(
            "last event: type {kind}, {event:?}, \"{}\"",
            text.trim_end_matches('\0')
        );
        if kind == DEBUG_EVENT_EXCEPTION {
            let record = exception.ExceptionRecord;
            println!(
                "exception {:#010x} at {:#x}",
                record.ExceptionCode.0, record.ExceptionAddress
            );
        }

        let (mut loaded, mut unloaded) = (0u32, 0u32);
        symbols.GetNumberModules(&mut loaded, &mut unloaded)?;
        let mut modules = Vec::new();
        for index in 0..loaded {
            let Ok(base) = symbols.GetModuleByIndex(index) else {
                continue;
            };
            let mut params = DEBUG_MODULE_PARAMETERS::default();
            let _ = symbols.GetModuleParameters(1, Some(&base), 0, &mut params);
            let mut name = [0u8; 260];
            let mut used = 0u32;
            let _ = symbols.GetModuleNameString(
                DEBUG_MODNAME_MODULE,
                index,
                0,
                Some(&mut name),
                Some(&mut used),
            );
            let name =
                String::from_utf8_lossy(&name[..used.saturating_sub(1) as usize]).into_owned();
            modules.push(Module {
                name,
                base,
                size: u64::from(params.Size),
            });
        }
        println!("{loaded} modules loaded, {unloaded} unloaded; first ones:");
        for m in modules.iter().take(8) {
            println!("  {:<24} {:#018x} {:>8} KB", m.name, m.base, m.size / 1024);
        }

        let ip = registers.GetInstructionOffset().unwrap_or(0);
        let sp = registers.GetStackOffset().unwrap_or(0);
        println!("ip {ip:#x} in {}; sp {sp:#x}", module_of(&modules, ip));

        let mut frames = [DEBUG_STACK_FRAME::default(); FRAMES];
        let mut filled = 0u32;
        match control.GetStackTrace(0, 0, 0, &mut frames, Some(&mut filled)) {
            Ok(()) => {
                println!("stack trace, {filled} frames:");
                for f in frames.iter().take(filled as usize) {
                    println!(
                        "  {:#018x} {}",
                        f.InstructionOffset,
                        module_of(&modules, f.InstructionOffset)
                    );
                }
            }
            Err(err) => println!("stack trace: {err}"),
        }

        let mut stack = vec![0u8; STACK_BYTES];
        let mut read = 0u32;
        let _ = data.ReadVirtual(
            sp,
            stack.as_mut_ptr().cast(),
            STACK_BYTES as u32,
            Some(&mut read),
        );
        let mut seen: Vec<&str> = Vec::new();
        for word in stack[..read as usize].as_chunks::<8>().0 {
            let value = u64::from_le_bytes(*word);
            if let Some(m) = modules
                .iter()
                .find(|m| value >= m.base && value < m.base + m.size)
                && !seen.contains(&m.name.as_str())
            {
                seen.push(&m.name);
            }
        }
        println!(
            "stack scan read {read} bytes; modules found, in order: {}",
            seen.join(", ")
        );
        let _ = client.EndSession(0);
    }

    for dll in [
        "symsrv.dll",
        "winhttp.dll",
        "wininet.dll",
        "dbghelp.dll",
        "dbgeng.dll",
    ] {
        // SAFETY: only asks whether the module is loaded; no reference is taken.
        let loaded = unsafe { GetModuleHandleW(&HSTRING::from(dll)) }.is_ok();
        println!("{dll}: {}", if loaded { "loaded" } else { "not loaded" });
    }
    let after = private_now(&mut own);
    println!(
        "total {:.0} ms; working set {} → {} MB, private {} → {} MB",
        started.elapsed().as_secs_f64() * 1000.0,
        before.0 >> 20,
        after.0 >> 20,
        before.1 >> 20,
        after.1 >> 20
    );
    Ok(())
}

fn module_of(modules: &[Module], address: u64) -> &str {
    modules
        .iter()
        .find(|m| address >= m.base && address < m.base + m.size)
        .map_or("?", |m| m.name.as_str())
}
