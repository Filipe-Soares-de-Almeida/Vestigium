use vestigium_core::health::BootReport;
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EVT_SYSTEM_PROPERTY_ID, EVT_VARIANT, EvtClose, EvtCreateRenderContext, EvtNext,
    EvtQuery, EvtQueryChannelPath, EvtQueryForwardDirection, EvtRenderContextSystem,
    EvtRenderEventValues, EvtRenderEventXml, EvtSystemEventID, EvtSystemTimeCreated,
};
use windows::core::HSTRING;

use crate::CollectError;
use crate::events::{FILETIME_TO_UNIX_MS, event_data, render, render_text};

/// Windows' boot performance log, which only an elevated process may read.
pub const BOOT_LOG: &str = "Microsoft-Windows-Diagnostics-Performance/Operational";
const BATCH: usize = 32;
const DAY_MS: u64 = 24 * 60 * 60_000;

/// An event log handle, closed when dropped.
struct Handle(EVT_HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from the event log API and is closed once, here.
        let _ = unsafe { EvtClose(self.0) };
    }
}

/// The boots Windows measured and the steps that slowed them over the last `days`; it needs
/// elevation, and reads only those ids and that span instead of the whole log.
pub fn boot_report(days: u32) -> Result<BootReport, CollectError> {
    let query = format!(
        "*[System[(EventID>=100 and EventID<=110) and TimeCreated[timediff(@SystemTime) <= {}]]]",
        u64::from(days) * DAY_MS
    );
    // SAFETY: a forward query over one channel; closed by `Handle`.
    let results = Handle(unsafe {
        EvtQuery(
            None,
            &HSTRING::from(BOOT_LOG),
            &HSTRING::from(query),
            EvtQueryChannelPath.0 | EvtQueryForwardDirection.0,
        )?
    });
    // SAFETY: a context for the system properties; closed by `Handle`.
    let context = Handle(unsafe { EvtCreateRenderContext(None, EvtRenderContextSystem.0)? });
    let mut report = BootReport::default();
    loop {
        let mut batch = [0isize; BATCH];
        let mut returned = 0u32;
        // SAFETY: `batch` receives up to BATCH event handles, each closed by `Handle` below.
        if unsafe { EvtNext(results.0, &mut batch, u32::MAX, 0, &mut returned) }.is_err() {
            break;
        }
        for raw in &batch[..(returned as usize).min(BATCH)] {
            let event = Handle(EVT_HANDLE(*raw));
            if let Some((id, ts_ms)) = id_and_time(context.0, event.0) {
                let xml = render_text(event.0, EvtRenderEventXml).unwrap_or_default();
                report.add(id, ts_ms, &event_data(&xml));
            }
        }
    }
    Ok(report)
}

/// The event's id and when it was logged, in Unix epoch milliseconds.
fn id_and_time(context: EVT_HANDLE, event: EVT_HANDLE) -> Option<(u16, i64)> {
    let (values, count) = render(Some(context), event, EvtRenderEventValues)?;
    // SAFETY: rendering system values fills the buffer with `count` variants, aligned by the u64 buffer.
    let variants = unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<EVT_VARIANT>(), count as usize)
    };
    let get = |id: EVT_SYSTEM_PROPERTY_ID| variants.get(usize::try_from(id.0).ok()?);
    // SAFETY: the system context renders the event id as a u16 and the time as a FILETIME.
    let (id, created) = unsafe {
        (
            get(EvtSystemEventID)?.Anonymous.UInt16Val,
            get(EvtSystemTimeCreated)?.Anonymous.FileTimeVal,
        )
    };
    Some((
        id,
        i64::try_from(created / 10_000).ok()? - FILETIME_TO_UNIX_MS,
    ))
}
