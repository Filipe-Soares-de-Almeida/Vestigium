//! Creates a test restore point the way the helper will before a batch of changes, and shows how to
//! tell a new point from one Windows skipped: `restore_probe [--twice]`, from an elevated terminal.

use std::thread::sleep;
use std::time::{Duration, Instant};

use windows::Win32::System::EventLog::{
    EVT_HANDLE, EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection,
    EvtRender, EvtRenderEventXml,
};
use windows::Win32::System::Restore::{
    BEGIN_SYSTEM_CHANGE, END_SYSTEM_CHANGE, MODIFY_SETTINGS, RESTOREPOINTINFO_EVENT_TYPE,
    RESTOREPOINTINFOW, SRSetRestorePointW, STATEMGRSTATUS,
};
use windows::core::HSTRING;

fn main() {
    let rounds = if std::env::args().any(|a| a == "--twice") {
        2
    } else {
        1
    };
    for round in 1..=rounds {
        println!("== round {round}");
        let before = last_8194();
        println!("last 8194 before: {before:?}");
        let started = Instant::now();
        let begin = set_point(BEGIN_SYSTEM_CHANGE, 0);
        println!(
            "BEGIN: ok {}, status {}, sequence {} ({:.0} ms)",
            begin.0,
            begin.1,
            begin.2,
            started.elapsed().as_secs_f64() * 1000.0
        );
        if begin.0 {
            let end = set_point(END_SYSTEM_CHANGE, begin.2);
            println!("END: ok {}, status {}, sequence {}", end.0, end.1, end.2);
        }
        sleep(Duration::from_secs(3));
        let after = last_8194();
        println!("last 8194 after: {after:?}; new point: {}", after != before);
    }
}

/// One SRSetRestorePointW call: whether it succeeded, its status and the sequence number it gave.
fn set_point(kind: RESTOREPOINTINFO_EVENT_TYPE, sequence: i64) -> (bool, u32, i64) {
    let mut text = [0u16; 256];
    for (slot, unit) in text
        .iter_mut()
        .zip("Vestigium test restore point".encode_utf16())
    {
        *slot = unit;
    }
    let info = RESTOREPOINTINFOW {
        dwEventType: kind,
        dwRestorePtType: MODIFY_SETTINGS,
        llSequenceNumber: sequence,
        szDescription: text,
    };
    let mut status = STATEMGRSTATUS::default();
    // SAFETY: both structs are the ones the call reads and fills.
    let ok = unsafe { SRSetRestorePointW(&info, &mut status) };
    (ok.as_bool(), status.nStatus.0, status.llSequenceNumber)
}

/// The time and record id of the newest "System Restore" 8194 event in the Application log.
fn last_8194() -> Option<(String, String)> {
    let query = "*[System[Provider[@Name='System Restore'] and EventID=8194]]";
    // SAFETY: a reverse query over one channel; closed below.
    let results = unsafe {
        EvtQuery(
            None,
            &HSTRING::from("Application"),
            &HSTRING::from(query),
            EvtQueryChannelPath.0 | EvtQueryReverseDirection.0,
        )
    }
    .ok()?;
    let mut batch = [0isize; 1];
    let mut returned = 0u32;
    // SAFETY: `batch` receives at most one event handle, closed below.
    let _ = unsafe { EvtNext(results, &mut batch, 2000, 0, &mut returned) };
    let mut found = None;
    if returned == 1 {
        let event = EVT_HANDLE(batch[0]);
        let mut buf = vec![0u16; 8192];
        let (mut used, mut count) = (0u32, 0u32);
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
            let field = |open: &str, close: char| {
                xml.split(open)
                    .nth(1)
                    .and_then(|s| s.split(close).next())
                    .map(str::to_owned)
            };
            found = field("SystemTime='", '\'').zip(field("<EventRecordID>", '<'));
        }
        // SAFETY: closes the event handle.
        let _ = unsafe { EvtClose(event) };
    }
    // SAFETY: closes the query handle.
    let _ = unsafe { EvtClose(results) };
    found
}
