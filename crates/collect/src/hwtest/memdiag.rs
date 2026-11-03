use vestigium_core::{CURATED, Channel, EventMeaning, WinEvent};
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EVT_SYSTEM_PROPERTY_ID, EVT_VARIANT, EvtCreateRenderContext, EvtNext, EvtQuery,
    EvtQueryChannelPath, EvtQueryForwardDirection, EvtRenderContextSystem, EvtRenderEventValues,
    EvtRenderEventXml, EvtSystemEventID, EvtSystemEventRecordId, EvtSystemLevel,
    EvtSystemTimeCreated,
};
use windows::core::HSTRING;

use crate::CollectError;
use crate::boot::Handle;
use crate::events::{FILETIME_TO_UNIX_MS, event_data, render, render_text};

const BATCH: usize = 16;

/// Every Windows Memory Diagnostic event the System log still holds, oldest first; needs no
/// elevation.
pub fn memory_diagnostic_events() -> Result<Vec<WinEvent>, CollectError> {
    let Some(entry) = CURATED
        .iter()
        .find(|e| e.meaning == EventMeaning::MemoryDiagnostic)
    else {
        return Ok(Vec::new());
    };
    let provider = entry.providers[0];
    let ids = entry
        .ids
        .iter()
        .map(|id| format!("EventID={id}"))
        .collect::<Vec<_>>()
        .join(" or ");
    let query = format!("*[System[Provider[@Name='{provider}'] and ({ids})]]");
    // SAFETY: a forward query over one channel; closed by `Handle`.
    let results = Handle(unsafe {
        EvtQuery(
            None,
            &HSTRING::from(Channel::System.path()),
            &HSTRING::from(query),
            EvtQueryChannelPath.0 | EvtQueryForwardDirection.0,
        )?
    });
    // SAFETY: a context for the system properties; closed by `Handle`.
    let context = Handle(unsafe { EvtCreateRenderContext(None, EvtRenderContextSystem.0)? });
    let mut events = Vec::new();
    loop {
        let mut batch = [0isize; BATCH];
        let mut returned = 0u32;
        // SAFETY: `batch` receives up to BATCH event handles, each closed by `Handle` below.
        if unsafe { EvtNext(results.0, &mut batch, u32::MAX, 0, &mut returned) }.is_err() {
            break;
        }
        for raw in &batch[..(returned as usize).min(BATCH)] {
            let event = Handle(EVT_HANDLE(*raw));
            if let Some(read) = read(context.0, event.0, provider) {
                events.push(read);
            }
        }
    }
    Ok(events)
}

fn read(context: EVT_HANDLE, event: EVT_HANDLE, provider: &str) -> Option<WinEvent> {
    let (values, count) = render(Some(context), event, EvtRenderEventValues)?;
    // SAFETY: rendering system values fills the buffer with `count` variants, aligned by the u64 buffer.
    let variants = unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<EVT_VARIANT>(), count as usize)
    };
    let get = |id: EVT_SYSTEM_PROPERTY_ID| variants.get(usize::try_from(id.0).ok()?);
    // SAFETY: the system context renders the id as a u16, the level as a byte, the rest as u64.
    let (id, level, created, record_id) = unsafe {
        (
            get(EvtSystemEventID)?.Anonymous.UInt16Val,
            get(EvtSystemLevel)?.Anonymous.ByteVal,
            get(EvtSystemTimeCreated)?.Anonymous.FileTimeVal,
            get(EvtSystemEventRecordId)?.Anonymous.UInt64Val,
        )
    };
    let xml = render_text(event, EvtRenderEventXml).unwrap_or_default();
    Some(WinEvent {
        ts_ms: i64::try_from(created / 10_000).ok()? - FILETIME_TO_UNIX_MS,
        channel: Channel::System,
        provider: provider.to_string(),
        id,
        record_id,
        level,
        meaning: EventMeaning::MemoryDiagnostic,
        data_xml: event_data(&xml),
    })
}
